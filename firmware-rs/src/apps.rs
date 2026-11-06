//! App ecosystem: streaming ustar tarball unpacker into LittleFS, app registry,
//! storage stats and default-app seeding.
//!
//! Rust port of `tar_unpacker.c`, the list/storage/delete parts of
//! `littlefs_driver.c`/`rest_api.c`, and `seed_apps.c` from the C firmware.
//! All LittleFS access goes through [`crate::storage::FS_MUTEX`]. Every
//! individual flash operation stays page-sized (writes) or one-sector
//! (erases): the `embassy-rp` flash driver runs each
//! `blocking_write`/`blocking_erase` inside a critical section with CORE1
//! paused, so short ops keep IRQ latency bounded.

use crate::storage::{self, with_fs, FlashStorage};
use core::fmt::Write as _;
use core::str::FromStr;
use heapless::String;
use littlefs2::fs::Filesystem;
use littlefs2::io::{Error as FsError, OpenSeekFrom};
use littlefs2::path::{Path, PathBuf};
use picoserve::io::Read as _;

/// Maximum accepted tarball size (byte cap for `POST /api/v1/apps/install`).
pub const MAX_TAR_BYTES: usize = 768 * 1024;

/// LittleFS path limit (paths must be ASCII, NUL-terminated, < 255 bytes).
const MAX_PATH_LEN: usize = 255;

/// Maximum number of path components accepted inside a tar entry.
const MAX_PATH_DEPTH: usize = 8;

/// Maximum length of a single app (directory) name.
pub const APP_NAME_MAX: usize = 32;

// ---------------------------------------------------------------------------
// AppParams: validated path parameter shared by all /apps routes
// ---------------------------------------------------------------------------

/// A validated app name (path segment). Anything failing [`FromStr`] falls
/// through to Picoserve's routing fallback (404), so invalid names never reach
/// the filesystem layer.
#[derive(Clone, PartialEq, Eq)]
pub struct AppParams(pub String<32>);

impl FromStr for AppParams {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_app_name(s)?;
        let mut buf = String::<32>::new();
        buf.push_str(s).map_err(|_| ())?;
        Ok(AppParams(buf))
    }
}

impl defmt::Format for AppParams {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(fmt, "{}", self.0.as_str());
    }
}

impl AppParams {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Security-critical: the name must be a bare path segment we can safely
/// concatenate onto `/apps/`.
pub fn validate_app_name(name: &str) -> Result<(), ()> {
    if name.is_empty() || name.len() > APP_NAME_MAX {
        return Err(());
    }
    if name == "." || name == ".." {
        return Err(());
    }
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            continue;
        }
        return Err(());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tar unpacker state machine (ustar, 512-byte blocks)
// ---------------------------------------------------------------------------

mod ustar {
    pub const NAME: usize = 0;
    pub const NAME_LEN: usize = 100;
    pub const SIZE: usize = 124;
    pub const SIZE_LEN: usize = 12;
    pub const TYPEFLAG: usize = 156;
    pub const PREFIX: usize = 345;
    pub const PREFIX_LEN: usize = 155;
    pub const BLOCK_LEN: usize = 512;
}

/// States of the streaming unpacker (`tar_unpacker.c` parity).
#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum UnpackerState {
    Header,
    Data,
    Padding,
    Done,
    Error,
}

/// Errors surfaced by the unpacker.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnpackError {
    NotMounted,
    BadEntryPath,
    BadArchiveName,
    LittleFs(FsError),
}

impl defmt::Format for UnpackError {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            UnpackError::NotMounted => defmt::write!(fmt, "not_mounted"),
            UnpackError::BadEntryPath => defmt::write!(fmt, "bad_entry_path"),
            UnpackError::BadArchiveName => defmt::write!(fmt, "bad_archive_name"),
            UnpackError::LittleFs(e) => defmt::write!(fmt, "littlefs:{}", defmt::Debug2Format(e)),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub enum TarTarget {
    App(String<32>),
    System,
}

/// Streaming ustar unpacker. Feed arbitrary byte chunks with [`Self::feed`];
/// the state machine buffers partial 512-byte headers internally, so the
/// caller may use any chunk size. Nothing is buffered beyond one block plus
/// bookkeeping. File data is written through LittleFS in page-sized
/// `write_chunk` calls (open/seek/write/close per chunk — no retained file
/// handle, no `unsafe`).
pub struct TarUnpacker {
    target: TarTarget,
    app: String<32>,
    app_dir: String<64>,
    state: UnpackerState,
    hdr_buf: [u8; ustar::BLOCK_LEN],
    hdr_bytes_have: usize,
    file_size: u32,
    file_bytes_written: u32,
    padding_needed: u32,
    padding_discarded: u32,
    /// Path of the file currently receiving data (Data state only).
    file_path: String<MAX_PATH_LEN>,
    pub total_files: u32,
    pub total_bytes: u32,
}

impl TarUnpacker {
    /// Create an unpacker targeting `/apps/{app}/...`. Creates the app
    /// directory fresh (removing any previous installation), so a failed
    /// install cannot mix with stale files.
    pub fn new(app: &AppParams) -> Result<Self, UnpackError> {
        let mut app_dir = String::<64>::new();
        write!(app_dir, "/apps/{}", app.as_str()).map_err(|_| UnpackError::BadArchiveName)?;

        let this = Self {
            target: TarTarget::App(app.0.clone()),
            app: app.0.clone(),
            app_dir,
            state: UnpackerState::Header,
            hdr_buf: [0; ustar::BLOCK_LEN],
            hdr_bytes_have: 0,
            file_size: 0,
            file_bytes_written: 0,
            padding_needed: 0,
            padding_discarded: 0,
            file_path: String::new(),
            total_files: 0,
            total_bytes: 0,
        };

        // Create /apps and the app dir fresh (truncate previous install).
        let root: &str = this.app_dir.as_str();
        let res = with_fs(|fs| -> Result<(), FsError> {
            match PathBuf::try_from(root) {
                Ok(app_path) => {
                    make_dir(fs, "/apps")?;
                    make_dir(fs, root)?;
                    match fs.remove_dir_all(&app_path) {
                        Ok(()) | Err(FsError::NO_SUCH_ENTRY) => make_dir(fs, root),
                        Err(e) => Err(e),
                    }
                }
                Err(_) => Err(FsError::IO),
            }
        });

        match res {
            Some(Ok(())) => {}
            Some(Err(e)) => return Err(UnpackError::LittleFs(e)),
            None => return Err(UnpackError::NotMounted),
        }
        Ok(this)
    }

    /// Create an unpacker targeting SYSTEM_FS root (`/`).
    pub fn new_system() -> Result<Self, UnpackError> {
        let this = Self {
            target: TarTarget::System,
            app: String::new(),
            app_dir: String::new(),
            state: UnpackerState::Header,
            hdr_buf: [0; ustar::BLOCK_LEN],
            hdr_bytes_have: 0,
            file_size: 0,
            file_bytes_written: 0,
            padding_needed: 0,
            padding_discarded: 0,
            file_path: String::new(),
            total_files: 0,
            total_bytes: 0,
        };

        let mounted = storage::with_system_fs(|_| ()).is_some();
        if !mounted {
            return Err(UnpackError::NotMounted);
        }
        Ok(this)
    }

    /// Feed bytes into the state machine. Returns the number of bytes
    /// consumed. After [`UnpackerState::Done`] or an error state, all further
    /// input is consumed and ignored (matching `tar_unpacker.c`).
    pub fn feed(&mut self, mut data: &[u8]) -> usize {
        let total_len = data.len();
        while !data.is_empty() {
            match self.state {
                UnpackerState::Header => {
                    let needed = ustar::BLOCK_LEN - self.hdr_bytes_have;
                    let take = data.len().min(needed);
                    self.hdr_buf[self.hdr_bytes_have..self.hdr_bytes_have + take]
                        .copy_from_slice(&data[..take]);
                    self.hdr_bytes_have += take;
                    data = &data[take..];

                    if self.hdr_bytes_have == ustar::BLOCK_LEN {
                        self.on_header();
                    }
                }
                UnpackerState::Data => {
                    let left = self.file_size - self.file_bytes_written;
                    let take = data.len().min(left as usize);
                    if take > 0 {
                        let chunk = &data[..take];
                        match self.write_file_chunk(chunk) {
                            Ok(()) => {
                                self.file_bytes_written += take as u32;
                                self.total_bytes += take as u32;
                            }
                            Err(_) => {
                                self.state = UnpackerState::Error;
                                break;
                            }
                        }
                        data = &data[take..];
                    }
                    if self.file_bytes_written >= self.file_size {
                        let rem = self.file_size % ustar::BLOCK_LEN as u32;
                        self.padding_needed = if rem != 0 {
                            ustar::BLOCK_LEN as u32 - rem
                        } else {
                            0
                        };
                        self.padding_discarded = 0;
                        self.file_path.clear();
                        self.state = if self.padding_needed > 0 {
                            UnpackerState::Padding
                        } else {
                            UnpackerState::Header
                        };
                    }
                }
                UnpackerState::Padding => {
                    let left = (self.padding_needed - self.padding_discarded) as usize;
                    let take = data.len().min(left);
                    self.padding_discarded += take as u32;
                    data = &data[take..];
                    if self.padding_discarded >= self.padding_needed {
                        self.state = UnpackerState::Header;
                    }
                }
                UnpackerState::Done | UnpackerState::Error => {
                    // Consume any remaining bytes without processing.
                    data = &[];
                }
            }
        }
        total_len
    }

    pub fn state(&self) -> UnpackerState {
        self.state
    }

    /// Handle a fully-buffered 512-byte header block.
    fn on_header(&mut self) {
        self.hdr_bytes_have = 0;

        // Zero block = end of archive marker.
        if self.hdr_buf.iter().all(|&b| b == 0) {
            self.state = UnpackerState::Done;
            return;
        }

        // Extract NUL-terminated name (+ ustar split prefix field if present).
        let mut raw = String::<MAX_PATH_LEN>::new();
        let name_end = self
            .hdr_buf[ustar::NAME..ustar::NAME + ustar::NAME_LEN]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(ustar::NAME_LEN);
        let prefix_end = self
            .hdr_buf[ustar::PREFIX..ustar::PREFIX + ustar::PREFIX_LEN]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(ustar::PREFIX_LEN);
        let has_prefix = prefix_end > 0;

        if has_prefix {
            // ustar: name = prefix/name when the prefix field is used.
            if from_ascii(&self.hdr_buf[ustar::PREFIX..ustar::PREFIX + prefix_end], &mut raw)
                .is_err()
                || raw.push('/').is_err()
                || from_ascii(&self.hdr_buf[..name_end], &mut raw).is_err()
            {
                self.state = UnpackerState::Error;
                return;
            }
        } else if from_ascii(&self.hdr_buf[..name_end], &mut raw).is_err() {
            self.state = UnpackerState::Error;
            return;
        }

        self.file_size = parse_octal(&self.hdr_buf[ustar::SIZE..ustar::SIZE + ustar::SIZE_LEN]);
        let typeflag = self.hdr_buf[ustar::TYPEFLAG] as char;

        // Sanitize the entry path relative to target root.
        let is_dir = typeflag == '5' || raw.ends_with('/');
        let is_system = matches!(self.target, TarTarget::System);

        let rel = if is_system {
            match sanitize_system_entry(&raw) {
                Ok(rel) => rel,
                Err(()) => {
                    defmt::warn!("system: rejected tar entry path (traversal guard)");
                    self.state = UnpackerState::Error;
                    return;
                }
            }
        } else {
            match sanitize_entry(&raw, self.app.as_str()) {
                Ok(rel) => rel,
                Err(()) => {
                    defmt::warn!("apps: rejected tar entry path (traversal guard)");
                    self.state = UnpackerState::Error;
                    return;
                }
            }
        };

        if rel.is_empty() {
            // Entry collapses to the root (".", app-name/, ...) — skip.
            return;
        }

        let mut full_path = String::<MAX_PATH_LEN>::new();
        if is_system {
            if full_path.push('/').is_err() || full_path.push_str(rel.as_str()).is_err() {
                self.state = UnpackerState::Error;
                return;
            }
        } else {
            if full_path.push_str(self.app_dir.as_str()).is_err()
                || full_path.push('/').is_err()
                || full_path.push_str(rel.as_str()).is_err()
            {
                self.state = UnpackerState::Error;
                return;
            }
        }

        let path = match PathBuf::try_from(full_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                self.state = UnpackerState::Error;
                return;
            }
        };

        let outcome = if is_system {
            storage::with_system_fs(|fs| -> Result<(), FsError> {
                if is_dir {
                    match fs.create_dir_all(&path) {
                        Ok(()) | Err(FsError::ENTRY_ALREADY_EXISTED) => Ok(()),
                        Err(e) => Err(e),
                    }
                } else {
                    fs.write(&path, &[])
                }
            })
        } else {
            let root_len = self.app_dir.len();
            let full: &str = full_path.as_str();
            with_fs(|fs| -> Result<(), FsError> {
                if is_dir {
                    match fs.create_dir_all(&path) {
                        Ok(()) | Err(FsError::ENTRY_ALREADY_EXISTED) => Ok(()),
                        Err(e) => Err(e),
                    }
                } else {
                    ensure_parents(fs, full, root_len)?;
                    // Create/truncate the destination file up front; data is then
                    // appended chunk-by-chunk with `write_chunk`.
                    fs.write(&path, &[])
                }
            })
        };

        match outcome {
            Some(Ok(())) => {
                if is_dir {
                    self.state = UnpackerState::Header;
                } else {
                    self.total_files += 1;
                    self.file_path = full_path;
                    self.file_bytes_written = 0;
                    self.state = if self.file_size > 0 {
                        UnpackerState::Data
                    } else {
                        UnpackerState::Header
                    };
                }
            }
            Some(Err(e)) => {
                defmt::warn!("tar entry failed: {}", defmt::Debug2Format(&e));
                self.state = UnpackerState::Error;
            }
            None => self.state = UnpackerState::Error,
        }
    }

    /// Write one chunk of the current file at the exact file offset.
    fn write_file_chunk(&self, chunk: &[u8]) -> Result<(), UnpackError> {
        let path = match PathBuf::try_from(self.file_path.as_str()) {
            Ok(p) => p,
            Err(_) => return Err(UnpackError::BadEntryPath),
        };
        let pos = OpenSeekFrom::Start(self.file_bytes_written);
        let res = match self.target {
            TarTarget::System => storage::with_system_fs(|fs| fs.write_chunk(&path, chunk, pos)),
            TarTarget::App(_) => with_fs(|fs| fs.write_chunk(&path, chunk, pos)),
        };
        res.ok_or(UnpackError::NotMounted)?
            .map_err(UnpackError::LittleFs)
    }
}

// --- helper: mkdir -----------------------------------------------------------

/// mkdir, tolerating an existing directory.
fn make_dir(fs: &Filesystem<'static, FlashStorage>, path: &str) -> Result<(), FsError> {
    match PathBuf::try_from(path) {
        Ok(p) => fs
            .create_dir(&p)
            .or_else(|e| if e == FsError::ENTRY_ALREADY_EXISTED { Ok(()) } else { Err(e) }),
        Err(_) => Err(FsError::IO),
    }
}

fn ensure_parents(
    fs: &Filesystem<'static, FlashStorage>,
    full: &str,
    root_len: usize,
) -> Result<(), FsError> {
    // mkdir -p for every component below the app root (C
    // `ensure_parent_dirs` parity).
    for i in (root_len + 1)..full.len() {
        if full.as_bytes()[i] == b'/' {
            make_dir(fs, &full[..i])?;
        }
    }
    Ok(())
}

// --- helper: ASCII / octal ---------------------------------------------------

fn from_ascii(bytes: &[u8], dst: &mut String<MAX_PATH_LEN>) -> Result<(), ()> {
    for &b in bytes {
        if b == 0 || b < 0x20 || b > 0x7e {
            return Err(());
        }
        dst.push(b as char).map_err(|_| ())?;
    }
    Ok(())
}

/// ustar octal size field: spaces, then octal digits, terminated by NUL/space.
fn parse_octal(field: &[u8]) -> u32 {
    let mut value: u32 = 0;
    let mut started = false;
    for &b in field {
        match b {
            b' ' | 0 => {
                if started {
                    break;
                }
            }
            b'0'..=b'7' => {
                started = true;
                value = value.wrapping_mul(8) + (b - b'0') as u32;
            }
            _ => break,
        }
    }
    value
}

/// Tar path sanitization (security-critical).
///
/// - rejects absolute paths, `..` components, backslashes, NUL/control bytes,
///   an embedded `apps/` first component (ecosystem namespace), and
///   path over-long / too deep entries;
/// - skips leading `.`/`/` characters and drops a first component that
///   matches the app dir basename (tarballs usually contain
///   `app-name/...`);
/// - returns the path *relative to the app dir* (may be empty to skip the
///   entry; no trailing slash — directory-ness is decided by the caller).
fn sanitize_entry(raw: &str, app_name: &str) -> Result<heapless::String<128>, ()> {
    if raw.contains('\\') || raw.contains('\0') {
        return Err(());
    }

    // C parity: `while (*norm == '.' || *norm == '/') norm++;`
    let norm = raw.trim_start_matches(['.', '/']);
    if norm.is_empty() {
        return Ok(heapless::String::new());
    }

    // Drop a leading component that matches the app dir basename.
    let norm = match norm.split_once('/') {
        Some((first, rest)) if first == app_name => rest,
        _ => norm,
    };

    if norm.is_empty() {
        return Ok(heapless::String::new());
    }

    // Reject the ecosystem namespace as a first component ("/apps/" guard).
    if norm == "apps" || norm.starts_with("apps/") {
        return Err(());
    }

    let mut rel = heapless::String::<128>::new();
    let mut depth = 0usize;
    for comp in norm.trim_end_matches('/').split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp == ".." || comp.len() > 64 {
            return Err(());
        }
        depth += 1;
        if depth > MAX_PATH_DEPTH {
            return Err(());
        }
        if !rel.is_empty() {
            rel.push('/').map_err(|_| ())?;
        }
        rel.push_str(comp).map_err(|_| ())?;
    }

    if rel.len() > MAX_PATH_LEN {
        return Err(());
    }
    Ok(rel)
}

/// System WebUI path sanitization.
fn sanitize_system_entry(raw: &str) -> Result<heapless::String<128>, ()> {
    if raw.contains('\\') || raw.contains('\0') {
        return Err(());
    }

    let norm = raw.trim_start_matches(['.', '/']);
    if norm.is_empty() {
        return Ok(heapless::String::new());
    }

    // Drop optional leading directory components like webui/ or dist/
    let norm = match norm.split_once('/') {
        Some((first, rest)) if first == "webui" || first == "dist" || first == "build" || first == "web-ui" => rest,
        _ => norm,
    };

    if norm.is_empty() {
        return Ok(heapless::String::new());
    }

    let mut rel = heapless::String::<128>::new();
    let mut depth = 0usize;
    for comp in norm.trim_end_matches('/').split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp == ".." || comp.len() > 64 {
            return Err(());
        }
        depth += 1;
        if depth > MAX_PATH_DEPTH {
            return Err(());
        }
        if !rel.is_empty() {
            rel.push('/').map_err(|_| ())?;
        }
        rel.push_str(comp).map_err(|_| ())?;
    }

    if rel.len() > MAX_PATH_LEN {
        return Err(());
    }
    Ok(rel)
}

// ---------------------------------------------------------------------------
// Registry: list, manifest, delete, storage stats
// ---------------------------------------------------------------------------

/// Recursively compute the byte size of a directory (parity with
/// `calc_dir_size_helper`). Depth-limited for safety.
fn dir_size(fs: &Filesystem<'static, FlashStorage>, path: &Path, depth: usize) -> u32 {
    if depth > 6 {
        return 0;
    }
    let mut total: u32 = 0;
    let _ = fs.read_dir_and_then(path, |entries| {
        for entry in entries.by_ref() {
            let Ok(entry) = entry else { continue };
            // Skip the "." (self) and ".." (parent) pseudo entries; without
            // this we would re-count this directory and every sibling app.
            let name = entry.file_name().as_str();
            if name == "." || name == ".." {
                continue;
            }
            let m = entry.metadata();
            if m.is_dir() {
                total = total.wrapping_add(dir_size(fs, entry.path(), depth + 1));
            } else {
                total = total.saturating_add(m.len() as u32);
            }
        }
        Ok(())
    });
    total
}

/// Manifest content is spliced into the JSON array only when it is bounded
/// printable-ASCII JSON with matching braces at both ends, so we never emit
/// broken JSON (the C firmware spliced blindly).
fn manifest_splicable(content: &[u8]) -> bool {
    content.first() == Some(&b'{') && content.last() == Some(&b'}')
}

const MANIFEST_CAP: usize = 256;

struct AppEntry {
    name: String<32>,
    size: u32,
    manifest: heapless::Vec<u8, MANIFEST_CAP>,
    has_manifest: bool,
}

/// `GET /api/v1/apps` — list installed apps as JSON into a bounded buffer
/// (no alloc). `None` when the filesystem is not mounted.
pub fn list_apps_json(buf: &mut String<1024>) -> Option<Result<(), UnpackError>> {
    let mut entries = heapless::Vec::<AppEntry, 16>::new();
    let mut first_err: Option<FsError> = None;

    let collected = with_fs(|fs| -> bool {
        // Snapshot /apps names first (no fs calls while iterating).
        let mut names = heapless::Vec::<String<32>, 16>::new();
        if let Ok(apps_path) = PathBuf::try_from("/apps") {
            let _ = fs.read_dir_and_then(apps_path.as_path(), |dir| {
                for entry in dir.by_ref() {
                    let Ok(entry) = entry else { continue };
                    if !entry.metadata().is_dir() {
                        continue;
                    }
                    let name = entry.file_name().as_str();
                    if name == "." || name == ".." {
                        continue;
                    }
                    let mut owned = String::<32>::new();
                    if owned.push_str(name).is_err() {
                        continue;
                    }
                    if names.push(owned).is_err() {
                        break;
                    }
                }
                Ok(())
            });
        }

        for name in names.iter() {
            let mut app_dir = String::<64>::new();
            let _ = write!(app_dir, "/apps/{}", name.as_str());
            let mut manifest_path = String::<64>::new();
            let _ = write!(manifest_path, "/apps/{}/app.json", name.as_str());

            let Ok(app_dir_path) = PathBuf::try_from(app_dir.as_str()) else {
                continue;
            };
            let size = dir_size(fs, app_dir_path.as_path(), 0);

            let mut manifest = heapless::Vec::<u8, MANIFEST_CAP>::new();
            let mut has_manifest = false;
            if let Ok(mp) = PathBuf::try_from(manifest_path.as_str()) {
                if let Ok(content) = fs.read::<MANIFEST_CAP>(&mp) {
                    has_manifest = manifest_splicable(&content);
                    manifest = content;
                }
            }

            if entries
                .push(AppEntry {
                    name: name.clone(),
                    size,
                    manifest,
                    has_manifest,
                })
                .is_err()
            {
                break;
            }
        }
        true
    });

    if collected != Some(true) {
        // `first_err` unused for now; treated as unmounted.
        let _ = &mut first_err;
        return None;
    }

    buf.clear();
    if buf.push('[').is_err() {
        return Some(Err(UnpackError::BadArchiveName));
    }
    let mut first = true;
    for entry in entries.iter() {
        if !first && buf.push(',').is_err() {
            return Some(Err(UnpackError::BadArchiveName));
        }
        first = false;

        let ok = if entry.has_manifest {
            // Splice: manifest is "{...}" — strip its final '}' and close the
            // object with the appended size/path fields.
            let body = &entry.manifest[..entry.manifest.len() - 1];
            let s = core::str::from_utf8(body).unwrap_or("");
            write!(
                buf,
                "{},\"size\":{},\"path\":\"/apps/{}/\"}}",
                s,
                entry.size,
                entry.name.as_str()
            )
        } else {
            write!(
                buf,
                "{{\"id\":\"{}\",\"name\":\"{}\",\"version\":\"1.0.0\",\"description\":\"Installed Application\",\"size\":{},\"path\":\"/apps/{}/\"}}",
                entry.name.as_str(),
                entry.name.as_str(),
                entry.size,
                entry.name.as_str()
            )
        };
        if ok.is_err() {
            return Some(Err(UnpackError::BadArchiveName));
        }
    }
    if buf.push(']').is_err() {
        return Some(Err(UnpackError::BadArchiveName));
    }
    Some(Ok(()))
}

/// `GET /api/v1/apps/{name}/manifest` — raw `app.json` content.
/// Outer `None` → unmounted; inner `None` → manifest missing.
pub fn read_manifest(app: &AppParams) -> Option<Result<Option<heapless::Vec<u8, MANIFEST_CAP>>, UnpackError>> {
    with_fs(|fs| {
        let mut manifest_path = String::<64>::new();
        if write!(manifest_path, "/apps/{}/app.json", app.as_str()).is_err() {
            return Ok(None);
        }
        match PathBuf::try_from(manifest_path.as_str()) {
            Ok(p) if fs.exists(&p) => match fs.read::<MANIFEST_CAP>(&p) {
                Ok(content) => Ok(Some(content)),
                Err(e) => Err(UnpackError::LittleFs(e)),
            },
            _ => Ok(None),
        }
    })
}

/// `DELETE /api/v1/apps/{name}` — recursively remove an app directory.
/// Outer `None` → unmounted; `Ok(())` → deleted; `Err(true)` → not found;
/// `Err(false)` → LittleFS error.
pub fn delete_app(app: &AppParams) -> Option<Result<(), bool>> {
    with_fs(|fs| {
        let mut app_path = String::<64>::new();
        let _ = write!(app_path, "/apps/{}", app.as_str());
        let p = match PathBuf::try_from(app_path.as_str()) {
            Ok(p) => p,
            Err(_) => return Err(true),
        };
        if !fs.exists(&p) {
            return Err(true);
        }
        match fs.remove_dir_all(&p) {
            Ok(()) => {
                defmt::info!("apps: deleted app {}", app.as_str());
                Ok(())
            }
            Err(e) => {
                defmt::warn!(
                    "apps: delete {} failed: {}",
                    app.as_str(),
                    defmt::Debug2Format(&e)
                );
                Err(false)
            }
        }
    })
}

/// Remove an app directory; used to clean up failed installs.
pub fn remove_app_dir_quiet(app: &str) {
    let mut app_path = String::<64>::new();
    let _ = write!(app_path, "/apps/{}", app);
    let _ = with_fs(|fs| {
        if let Ok(p) = PathBuf::try_from(app_path.as_str()) {
            let _ = fs.remove_dir_all(&p);
        }
    });
}

/// Storage statistics for `GET /api/v1/apps/storage`. `None` → unmounted.
pub struct StorageStats {
    pub total_bytes: u32,
    pub used_bytes: u32,
    pub free_bytes: u32,
    pub block_size: u32,
    pub block_count: u32,
}

pub fn storage_stats() -> Option<Result<StorageStats, UnpackError>> {
    with_fs(|fs| {
        let total = fs.total_space() as u32;
        // `available_space()` is a lower bound of free space (littlefs
        // "current size" semantics); used = total - free.
        match fs.available_space() {
            Ok(free) => {
                let free = free.min(total as usize) as u32;
                Ok(StorageStats {
                    total_bytes: total,
                    used_bytes: total - free,
                    free_bytes: free,
                    block_size: 4096,
                    block_count: total / 4096,
                })
            }
            Err(e) => Err(UnpackError::LittleFs(e)),
        }
    })
}

// ---------------------------------------------------------------------------
// Tarball install endpoint plumbing (POST /api/v1/apps/install/{name})
// ---------------------------------------------------------------------------

/// Successful install summary.
pub struct InstallReport {
    pub app: String<32>,
    pub files: u32,
    pub bytes: u32,
}

/// Install failure kinds surfaced through the REST endpoint.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InstallError {
    /// Body exceeded [`MAX_TAR_BYTES`].
    TooLarge,
    /// LittleFS is not mounted.
    NotMounted,
    /// Body ended before the tar end-of-archive marker.
    Incomplete,
    /// Socket error while consuming the body.
    Io,
    /// Unpacker/filesystem failure.
    Failed(UnpackError),
}

/// Stream the request body into the tar unpacker and write the app to
/// LittleFS. The body is read in 1 KiB chunks; the request read timeout is
/// lifted to 60 s for the duration (the default RUDY-protection timeout is
/// 3 s and would abort multi-second uploads).
///
/// On any failure the partially-written app directory is removed, so an
/// aborted install never leaves an inconsistent app.
pub async fn install_tarball<R: picoserve::io::Read>(
    app: &AppParams,
    body: picoserve::request::RequestBody<'_, R>,
) -> Result<InstallReport, InstallError> {
    // Cheap early check: refuse oversized proven bodies before any I/O.
    if body.content_length() > MAX_TAR_BYTES {
        return Err(InstallError::TooLarge);
    }

    let mut unpacker = match TarUnpacker::new(app) {
        Ok(u) => u,
        Err(UnpackError::NotMounted) => return Err(InstallError::NotMounted),
        Err(e) => return Err(InstallError::Failed(e)),
    };

    let mut reader = body
        .reader()
        .with_different_timeout(embassy_time::Duration::from_secs(60));

    let mut buf = [0u8; 1024];
    let mut total = 0usize;

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let _consumed = unpacker.feed(&buf[..n]);
                total += n;
                if total > MAX_TAR_BYTES {
                    remove_app_dir_quiet(app.as_str());
                    return Err(InstallError::TooLarge);
                }
                if unpacker.state() == UnpackerState::Done {
                    break;
                }
            }
            Err(_) => {
                remove_app_dir_quiet(app.as_str());
                return Err(InstallError::Io);
            }
        }
    }

    if unpacker.state() != UnpackerState::Done {
        remove_app_dir_quiet(app.as_str());
        return Err(InstallError::Incomplete);
    }

    defmt::info!(
        "apps: installed app {} ({} files, {} bytes)",
        app.as_str(),
        unpacker.total_files,
        unpacker.total_bytes
    );

    Ok(InstallReport {
        app: app.0.clone(),
        files: unpacker.total_files,
        bytes: unpacker.total_bytes,
    })
}

/// WebUI update summary.
pub struct WebUiUpdateReport {
    pub files: u32,
    pub bytes: u32,
}

/// Stream request body into the tar unpacker and write System WebUI to SYSTEM_FS.
pub async fn update_system_webui<R: picoserve::io::Read>(
    body: picoserve::request::RequestBody<'_, R>,
) -> Result<WebUiUpdateReport, InstallError> {
    if body.content_length() > storage::SYSTEM_FS_SIZE_BYTES {
        return Err(InstallError::TooLarge);
    }

    let mut unpacker = match TarUnpacker::new_system() {
        Ok(u) => u,
        Err(UnpackError::NotMounted) => return Err(InstallError::NotMounted),
        Err(e) => return Err(InstallError::Failed(e)),
    };

    let mut reader = body
        .reader()
        .with_different_timeout(embassy_time::Duration::from_secs(60));

    let mut buf = [0u8; 1024];
    let mut total = 0usize;

    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let _consumed = unpacker.feed(&buf[..n]);
                total += n;
                if total > storage::SYSTEM_FS_SIZE_BYTES {
                    return Err(InstallError::TooLarge);
                }
                if unpacker.state() == UnpackerState::Done {
                    break;
                }
            }
            Err(_) => {
                return Err(InstallError::Io);
            }
        }
    }

    if unpacker.state() != UnpackerState::Done {
        return Err(InstallError::Incomplete);
    }

    defmt::info!(
        "system: updated System WebUI ({} files, {} bytes)",
        unpacker.total_files,
        unpacker.total_bytes
    );

    Ok(WebUiUpdateReport {
        files: unpacker.total_files,
        bytes: unpacker.total_bytes,
    })
}

// ---------------------------------------------------------------------------
// Seeding (port of seed_apps.c, minimized per proposal §3.2)
// ---------------------------------------------------------------------------

const SERVO_MASTER_APP_JSON: &str = r#"{"name":"servo-master","title":"Servo Master","version":"1.0.0","author":"AgentLA Team","description":"Servo control demo app","category":"Robotics","entry":"index.html"}"#;

const SERVO_MASTER_INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Servo Master</title></head>
<body><h2>Servo Master</h2><p>Seeded demo app from AgentLA firmware.</p>
<p><a href="/">Back to workbench</a></p></body></html>"#;

/// Ensure `/apps` exists and seed the default `servo-master` app when its
/// manifest is missing (matches C `seed_default_apps`: checked on every boot,
/// so the seed is restored even if a user deletes it).
pub fn seed_default_apps() {
    let _ = with_fs(|fs| -> Result<(), FsError> {
        make_dir(fs, "/apps")?;
        let servo_dir = "/apps/servo-master";
        make_dir(fs, servo_dir)?;

        match PathBuf::try_from("/apps/servo-master/app.json") {
            Ok(manifest) if fs.exists(&manifest) => Ok(()), // already seeded
            Ok(manifest) => {
                defmt::info!("apps: seeding default app servo-master");
                if let Err(e) = fs.write(&manifest, SERVO_MASTER_APP_JSON.as_bytes()) {
                    defmt::warn!(
                        "apps: seed app.json failed: {}",
                        defmt::Debug2Format(&e)
                    );
                }
                if let Ok(html) = PathBuf::try_from("/apps/servo-master/index.html") {
                    if let Err(e) = fs.write(&html, SERVO_MASTER_INDEX_HTML.as_bytes()) {
                        defmt::warn!(
                            "apps: seed index.html failed: {}",
                            defmt::Debug2Format(&e)
                        );
                    }
                }
                Ok(())
            }
            Err(_) => Ok(()),
        }
    });
}
