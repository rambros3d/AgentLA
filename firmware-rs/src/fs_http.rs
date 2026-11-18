//! Serving of uploaded app files out of LittleFS and the streaming tarball
//! installer endpoints, plus dynamic streaming of System WebUI assets from
//! SYSTEM_FS with transparent Gzip decompression / Content-Encoding: gzip.

use core::fmt::Write as _;

use littlefs2::io::OpenSeekFrom;
use littlefs2::path::PathBuf;
use picoserve::io::Read;
use picoserve::request::{Path, Request};
use picoserve::response::chunked::{ChunkWriter, ChunkedResponse, Chunks, ChunksWritten};
use picoserve::response::{IntoResponse, ResponseWriter, StatusCode};
use picoserve::routing::PathRouterService;
use picoserve::ResponseSent;

use crate::apps::{self, AppParams};
use crate::storage::{
    FsAccessError, with_fs, with_system_fs, with_system_fs_result, with_user_fs_result,
};

#[path = "fs_json.rs"]
mod fs_json;
use fs_json::{write_dir_entry, write_json_string};

/// Size of a single LittleFS read/write chunk served or stored per lock hold.
pub const FS_CHUNK: usize = 2048;

// ---------------------------------------------------------------------------
// MIME handling
// ---------------------------------------------------------------------------

pub fn mime_for(path: &str) -> &'static str {
    let Some(dot) = path.rsplit_once('.') else {
        return "application/octet-stream";
    };
    let ext = dot.1;
    match ext {
        "html" | "htm" => "text/html",
        "js" | "mjs" => "application/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        "txt" => "text/plain",
        "md" | "markdown" => "text/markdown; charset=utf-8",
        _ => "application/octet-stream",
    }
}

// ---------------------------------------------------------------------------
// Streaming file body from LittleFS (USER_FS)
// ---------------------------------------------------------------------------

/// A [`Chunks`] impl that streams one LittleFS file in 512-byte chunks.
pub struct AppFileChunks {
    path: PathBuf,
    mime: &'static str,
}

impl Chunks for AppFileChunks {
    fn content_type(&self) -> &'static str {
        self.mime
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let mut pos: u32 = 0;
        loop {
            let read = with_fs(|fs| {
                fs.read_chunk::<FS_CHUNK>(&self.path, OpenSeekFrom::Start(pos))
                    .ok()
            });
            let Some((chunk, _file_len)) = read.flatten() else { break };
            if chunk.is_empty() {
                break;
            }
            writer.write_chunk(chunk.as_slice()).await?;
            pos += chunk.len() as u32;
        }
        writer.finalize().await
    }
}

/// Build the LittleFS path `/apps/{name}/{rest...}` from the URL-decoded segments.
/// Defaults to `/apps/{name}/index.html` if no path or a trailing directory slash was supplied.
fn app_fs_path(app: &AppParams, rest: &Path<'_>) -> Option<PathBuf> {
    let mut path = heapless::String::<255>::new();
    write!(path, "/apps/{}", app.as_str()).ok()?;
    let mut segments = rest.segments();
    let mut any = false;
    while let Some(seg) = segments.next() {
        let decoded = seg.try_into_string::<255>().ok()?;
        if decoded.is_empty() {
            continue; // collapses `a//b`
        }
        if decoded.contains('\\') || decoded.contains('\0') {
            return None;
        }
        for comp in decoded.split('/') {
            if comp.is_empty() || comp == "." {
                continue;
            }
            if comp == ".." {
                return None; // traversal guard
            }
            if !path.is_empty() && !path.ends_with('/') {
                path.push('/').ok()?;
            }
            path.push_str(comp).ok()?;
        }
        any = true;
    }
    if !any || path.ends_with('/') {
        if !path.ends_with('/') {
            path.push('/').ok()?;
        }
        path.push_str("index.html").ok()?;
    }
    PathBuf::try_from(path.as_str()).ok()
}

/// `nest_service` handler for `/apps/{name}/{*path}`: serve files straight from LittleFS.
pub struct AppFiles;

impl PathRouterService<(), (AppParams,)> for AppFiles {
    async fn call_path_router_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (name,): (AppParams,),
        path: Path<'_>,
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if request.parts.method() != "GET" && request.parts.method() != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let path_ = match app_fs_path(&name, &path) {
            Some(p) => p,
            None => {
                return not_found(request, response_writer).await;
            }
        };

        // If the path points to a directory in LittleFS, try index.html inside it
        let path_ = if with_fs(|fs| fs.metadata(&path_).map(|m| m.is_dir()).unwrap_or(false)).unwrap_or(false)
            || with_system_fs(|fs| fs.metadata(&path_).map(|m| m.is_dir()).unwrap_or(false)).unwrap_or(false)
        {
            let mut s = heapless::String::<255>::new();
            let _ = write!(s, "{}/index.html", path_.as_str().trim_end_matches('/'));
            PathBuf::try_from(s.as_str()).unwrap_or(path_)
        } else {
            path_
        };

        let is_user_file = with_fs(|fs| fs.metadata(&path_).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);

        if is_user_file {
            let mime = mime_for(path_.as_str());
            let is_html = path_.as_str().ends_with(".html") || mime.contains("html");
            let cache_ctrl = if is_html {
                "no-cache, must-revalidate"
            } else {
                "public, max-age=604800, immutable"
            };
            let chunks = AppFileChunks { path: path_, mime };

            return ChunkedResponse::new(chunks)
                .into_response()
                .with_header("Cache-Control", cache_ctrl)
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let is_sys_file = with_system_fs(|fs| fs.metadata(&path_).map(|m| m.is_file()).unwrap_or(false))
            .unwrap_or(false);

        if is_sys_file {
            let mime = mime_for(path_.as_str());
            let is_html = path_.as_str().ends_with(".html") || mime.contains("html");
            let cache_ctrl = if is_html {
                "no-cache, must-revalidate"
            } else {
                "public, max-age=604800, immutable"
            };
            let chunks = SystemFileChunks { path: path_, mime };

            return ChunkedResponse::new(chunks)
                .into_response()
                .with_header("Cache-Control", cache_ctrl)
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        not_found(request, response_writer).await
    }
}

// ---------------------------------------------------------------------------
// Dynamic Streaming file body from SYSTEM_FS (WebUI partition)
// ---------------------------------------------------------------------------

/// A [`Chunks`] impl that streams one file from SYSTEM_FS in 512-byte chunks.
pub struct SystemFileChunks {
    path: PathBuf,
    mime: &'static str,
}

impl Chunks for SystemFileChunks {
    fn content_type(&self) -> &'static str {
        self.mime
    }

    async fn write_chunks<W: picoserve::io::Write>(
        self,
        mut writer: ChunkWriter<W>,
    ) -> Result<ChunksWritten, W::Error> {
        let mut pos: u32 = 0;
        loop {
            let read = with_system_fs(|fs| {
                fs.read_chunk::<FS_CHUNK>(&self.path, OpenSeekFrom::Start(pos))
                    .ok()
            });
            let Some((chunk, _file_len)) = read.flatten() else { break };
            if chunk.is_empty() {
                break;
            }
            writer.write_chunk(chunk.as_slice()).await?;
            pos += chunk.len() as u32;
        }
        writer.finalize().await
    }
}

/// Request handler for dynamically serving System WebUI files out of Partition 1 (`SYSTEM_FS`).
/// Supports transparent pre-compressed gzip streaming with `Content-Encoding: gzip`
/// and redirects to /device if partition is unpopulated or corrupt.
pub struct SystemWebFile {
    file_name: &'static str,
    mime: &'static str,
}

impl SystemWebFile {
    pub const fn new(file_name: &'static str, mime: &'static str) -> Self {
        Self { file_name, mime }
    }
}

impl picoserve::routing::RequestHandlerService<(), ()> for SystemWebFile {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if request.parts.method() != "GET" && request.parts.method() != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let client_accepts_gzip = request
            .parts
            .headers()
            .iter()
            .any(|(name, value)| {
                name == "accept-encoding"
                    && value.as_str().map(|v| v.contains("gzip")).unwrap_or(false)
            });

        // 1. If client accepts gzip, prioritize pre-compressed `.gz` asset from SYSTEM_FS
        let mut gz_name = heapless::String::<64>::new();
        let _ = write!(gz_name, "/{}.gz", self.file_name);
        let gz_path_opt = PathBuf::try_from(gz_name.as_str()).ok();

        let mut raw_name = heapless::String::<64>::new();
        let _ = write!(raw_name, "/{}", self.file_name);
        let raw_path_opt = PathBuf::try_from(raw_name.as_str()).ok();

        let is_html = self.file_name.ends_with(".html") || self.mime.contains("html");
        let cache_ctrl = if is_html {
            "no-cache, must-revalidate"
        } else {
            "public, max-age=604800, immutable"
        };

        if client_accepts_gzip {
            if let Some(ref gz_path) = gz_path_opt {
                let exists = with_system_fs(|fs| {
                    fs.metadata(gz_path).map(|m| m.is_file()).unwrap_or(false)
                })
                .unwrap_or(false);

                if exists {
                    let chunks = SystemFileChunks {
                        path: gz_path.clone(),
                        mime: self.mime,
                    };
                    let resp = ChunkedResponse::new(chunks).into_response();
                    let resp = resp.with_header("Content-Encoding", "gzip");
                    let resp = resp.with_header("Cache-Control", cache_ctrl);
                    return resp
                        .write_to(request.body_connection.finalize().await?, response_writer)
                        .await;
                }
            }
        }

        // 2. Try raw uncompressed file from SYSTEM_FS
        if let Some(ref raw_path) = raw_path_opt {
            let exists = with_system_fs(|fs| {
                fs.metadata(raw_path).map(|m| m.is_file()).unwrap_or(false)
            })
            .unwrap_or(false);

            if exists {
                let chunks = SystemFileChunks {
                    path: raw_path.clone(),
                    mime: self.mime,
                };
                let resp = ChunkedResponse::new(chunks).into_response();
                let resp = resp.with_header("Cache-Control", cache_ctrl);
                return resp
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // 3. Fallback: serve .gz asset even if client omitted Accept-Encoding (HTTP/1.1 compatible)
        if let Some(ref gz_path) = gz_path_opt {
            let exists = with_system_fs(|fs| {
                fs.metadata(gz_path).map(|m| m.is_file()).unwrap_or(false)
            })
            .unwrap_or(false);

            if exists {
                let chunks = SystemFileChunks {
                    path: gz_path.clone(),
                    mime: self.mime,
                };
                let resp = ChunkedResponse::new(chunks).into_response();
                let resp = resp.with_header("Content-Encoding", "gzip");
                let resp = resp.with_header("Cache-Control", cache_ctrl);
                return resp
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        }

        // 4. Fallback recovery for HTML pages if partition is unpopulated or corrupt -> redirect to /device
        if self.file_name == "index.html" || self.file_name == "lab.html" || self.file_name == "terminal.html" {
            return (
                StatusCode::FOUND,
                ("Location", "/device"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting to /device...",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        not_found(request, response_writer).await
    }
}

async fn not_found<R: Read, W: ResponseWriter<Error = R::Error>>(
    request: Request<'_, R>,
    response_writer: W,
) -> Result<ResponseSent, W::Error> {
    (
        StatusCode::NOT_FOUND,
        ("Content-Type", "application/json"),
        "{\"status\":\"not_found\"}",
    )
        .write_to(request.body_connection.finalize().await?, response_writer)
        .await
}

// ---------------------------------------------------------------------------
// Tarball install endpoint: POST /api/v1/apps/install/{name}
// ---------------------------------------------------------------------------

pub struct AppInstall;

impl picoserve::routing::MethodHandlerService<(), (AppParams,)> for AppInstall {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (name,): (AppParams,),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let report = apps::install_tarball(&name, request.body_connection.body()).await;

        let mut body = heapless::String::<128>::new();
        let mut code = StatusCode::OK;
        match report {
            Ok(r) => {
                let _ = write!(
                    body,
                    r#"{{"status":"installed","app":"{}","files":{},"bytes":{}}}"#,
                    r.app.as_str(),
                    r.files,
                    r.bytes
                );
            }
            Err(e) => {
                let _ = write!(
                    body,
                    "{}",
                    match e {
                        apps::InstallError::TooLarge => "{\"error\":\"payload_too_large\"}",
                        apps::InstallError::NotMounted => "{\"error\":\"fs_not_mounted\"}",
                        apps::InstallError::Incomplete => "{\"error\":\"incomplete_archive\"}",
                        apps::InstallError::Io | apps::InstallError::Failed(_) => {
                            "{\"error\":\"install_failed\"}"
                        }
                    }
                );
                code = match e {
                    apps::InstallError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    apps::InstallError::NotMounted => StatusCode::SERVICE_UNAVAILABLE,
                    apps::InstallError::Incomplete => StatusCode::BAD_REQUEST,
                    apps::InstallError::Io | apps::InstallError::Failed(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
            }
        };

        (code, ("Content-Type", "application/json"), body.as_str())
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

/// Tarball install endpoint handling query parameter style: POST /api/v1/apps/install?name={name}
pub struct AppInstallQuery;

impl picoserve::routing::RequestHandlerService<(), ()> for AppInstallQuery {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        let query = request.parts.query();
        let name_str = match crate::rest::query_val(query, "name") {
            Some(n) => n,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_name_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let name = match core::str::FromStr::from_str(name_str.as_str()) {
            Ok(n) => n,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_app_name\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let report = apps::install_tarball(&name, request.body_connection.body()).await;

        let mut body = heapless::String::<128>::new();
        let mut code = StatusCode::OK;
        match report {
            Ok(r) => {
                let _ = write!(
                    body,
                    r#"{{"status":"installed","app":"{}","files":{},"bytes":{}}}"#,
                    r.app.as_str(),
                    r.files,
                    r.bytes
                );
            }
            Err(e) => {
                let _ = write!(
                    body,
                    "{}",
                    match e {
                        apps::InstallError::TooLarge => "{\"error\":\"payload_too_large\"}",
                        apps::InstallError::NotMounted => "{\"error\":\"fs_not_mounted\"}",
                        apps::InstallError::Incomplete => "{\"error\":\"incomplete_archive\"}",
                        apps::InstallError::Io | apps::InstallError::Failed(_) => {
                            "{\"error\":\"install_failed\"}"
                        }
                    }
                );
                code = match e {
                    apps::InstallError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    apps::InstallError::NotMounted => StatusCode::SERVICE_UNAVAILABLE,
                    apps::InstallError::Incomplete => StatusCode::BAD_REQUEST,
                    apps::InstallError::Io | apps::InstallError::Failed(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
            }
        }

        (code, ("Content-Type", "application/json"), body.as_str())
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

// ---------------------------------------------------------------------------
// System WebUI Live Update endpoint: POST /api/v1/system/webui/update
// ---------------------------------------------------------------------------

pub struct SystemWebUiUpdate;

impl picoserve::routing::MethodHandlerService<()> for SystemWebUiUpdate {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let report = apps::update_system_webui(request.body_connection.body()).await;

        let mut body = heapless::String::<128>::new();
        let mut code = StatusCode::OK;
        match report {
            Ok(r) => {
                let _ = write!(
                    body,
                    r#"{{"status":"updated","files":{},"bytes":{}}}"#,
                    r.files,
                    r.bytes
                );
            }
            Err(e) => {
                let _ = write!(
                    body,
                    "{}",
                    match e {
                        apps::InstallError::TooLarge => "{\"error\":\"payload_too_large\"}",
                        apps::InstallError::NotMounted => "{\"error\":\"fs_not_mounted\"}",
                        apps::InstallError::Incomplete => "{\"error\":\"incomplete_archive\"}",
                        apps::InstallError::Io | apps::InstallError::Failed(_) => {
                            "{\"error\":\"update_failed\"}"
                        }
                    }
                );
                code = match e {
                    apps::InstallError::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
                    apps::InstallError::NotMounted => StatusCode::SERVICE_UNAVAILABLE,
                    apps::InstallError::Incomplete => StatusCode::BAD_REQUEST,
                    apps::InstallError::Io | apps::InstallError::Failed(_) => {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                };
            }
        };

        (code, ("Content-Type", "application/json"), body.as_str())
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}



// ---------------------------------------------------------------------------
// Filesystem REST API Endpoints: /api/v1/fs/*
// ---------------------------------------------------------------------------

pub fn decode_url_percent(input: &str) -> heapless::String<128> {
    let mut out = heapless::String::<128>::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex_slice = match core::str::from_utf8(&bytes[i + 1..i + 3]) {
                Ok(s) => s,
                Err(_) => {
                    let _ = out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
            };
            if let Ok(val) = u8::from_str_radix(hex_slice, 16) {
                let _ = out.push(val as char);
                i += 3;
                continue;
            }
        } else if bytes[i] == b'+' {
            let _ = out.push(' ');
            i += 1;
            continue;
        }
        let _ = out.push(bytes[i] as char);
        i += 1;
    }
    out
}

pub fn sanitize_fs_path(raw: &str) -> Option<heapless::String<128>> {
    let decoded = decode_url_percent(raw);
    let s = decoded.as_str().trim();
    if s.contains('\\') || s.contains('\0') {
        return None;
    }
    for seg in s.split('/') {
        if seg == ".." {
            return None;
        }
    }
    let mut clean = heapless::String::<128>::new();
    if !s.starts_with('/') {
        let _ = clean.push('/');
    }
    let mut last_was_slash = clean.ends_with('/');
    for ch in s.chars() {
        if ch == '/' {
            if !last_was_slash {
                let _ = clean.push('/');
                last_was_slash = true;
            }
        } else {
            let _ = clean.push(ch);
            last_was_slash = false;
        }
    }
    if clean.len() > 1 && clean.ends_with('/') {
        let _ = clean.pop();
    }
    if clean.is_empty() {
        let _ = clean.push('/');
    }
    Some(clean)
}

fn ensure_parent_dirs_user(
    fs: &littlefs2::fs::Filesystem<'static, crate::storage::UserFlashStorage>,
    full: &str,
) {
    let bytes = full.as_bytes();
    for i in 1..bytes.len() {
        if bytes[i] == b'/' {
            if let Ok(dir_path) = PathBuf::try_from(&full[..i]) {
                let _ = fs.create_dir_all(&dir_path);
            }
        }
    }
}

/// Why a directory listing could not be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirListError {
    /// The target directory does not exist or littlefs reported an I/O error.
    Io,
    /// The entry array does not fit in the response buffer. Never emit a
    /// partial array in this case; the caller returns `listing_too_large`.
    TooLarge,
}

/// Maximum size of a serialised directory listing. A single
/// [`heapless::String`] buffer with checked writes is used instead of streaming
/// because littlefs2 only exposes its directory iterator through a synchronous
/// closure (see design.md, task 1.1). Overflow is reported, never truncated.
const DIR_LIST_CAPACITY: usize = 8192;

type DirListBuffer = heapless::String<DIR_LIST_CAPACITY>;

/// Append every entry of `target` to `out` as JSON objects.
///
/// `first` tracks comma placement across calls. An overflow while writing any
/// entry aborts the walk and is reported as [`DirListError::TooLarge`] so the
/// response can fail explicitly rather than returning truncated JSON.
fn append_dir_entries<S: littlefs2::driver::Storage>(
    fs: &littlefs2::fs::Filesystem<'static, S>,
    target: &littlefs2::path::Path,
    out: &mut DirListBuffer,
    first: &mut bool,
) -> Result<(), DirListError> {
    let mut overflow = false;
    let res = fs.read_dir_and_then(target, |dir| {
        for entry in dir.by_ref() {
            let Ok(entry) = entry else { continue };
            let name = entry.file_name().as_str();
            if name == "." || name == ".." {
                continue;
            }
            let m = entry.metadata();
            let is_dir = m.is_dir();
            let size = if is_dir { 0 } else { m.len() };
            if write_dir_entry(out, first, name, is_dir, size).is_err() {
                overflow = true;
                break;
            }
        }
        Ok(())
    });

    if overflow {
        Err(DirListError::TooLarge)
    } else {
        res.map_err(|_| DirListError::Io)
    }
}

/// Helper to serialize directory entries into a JSON array buffer.
///
/// The array is either complete or this returns an error. In particular, an
/// exhausted buffer never produces a syntactically valid but truncated `]`.
fn serialize_dir_entries<S: littlefs2::driver::Storage>(
    fs: &littlefs2::fs::Filesystem<'static, S>,
    path_str: &str,
    out: &mut DirListBuffer,
) -> Result<(), DirListError> {
    let target_path = if path_str == "/" || path_str.is_empty() {
        PathBuf::try_from(".").map_err(|_| DirListError::Io)?
    } else {
        PathBuf::try_from(path_str).map_err(|_| DirListError::Io)?
    };

    let mut first = true;
    out.push('[').map_err(|_| DirListError::TooLarge)?;
    let mut result = append_dir_entries(fs, &target_path, out, &mut first);

    // Preserve the historical fallback: some drives reject "." but accept "/".
    if matches!(result, Err(DirListError::Io)) && (path_str == "/" || path_str.is_empty()) {
        if let Ok(root_path) = PathBuf::try_from("/") {
            out.clear();
            first = true;
            out.push('[').map_err(|_| DirListError::TooLarge)?;
            result = append_dir_entries(fs, &root_path, out, &mut first);
        }
    }

    // Only finalise a successful, complete array.
    result?;
    out.push(']').map_err(|_| DirListError::TooLarge)?;
    Ok(())
}

/// GET /api/v1/fs/stat
pub struct FsStat;

impl picoserve::routing::MethodHandlerService<()> for FsStat {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        // A contended partition is a transient 503, not a cached-success or a
        // not-mounted condition. Probe before the cached usage helpers below.
        if crate::storage::system_fs_busy() || crate::storage::user_fs_busy() {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                ("Content-Type", "application/json"),
                "{\"error\":\"fs_busy\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let (sys_used, sys_total) = crate::storage::system_fs_usage();
        let (usr_used, usr_total) = crate::storage::user_fs_usage();

        let mut body = heapless::String::<256>::new();
        let _ = write!(
            body,
            r#"{{"system":{{"total_bytes":{},"used_bytes":{},"read_only":true}},"user":{{"total_bytes":{},"used_bytes":{},"read_only":false}}}}"#,
            sys_total, sys_used, usr_total, usr_used
        );

        let finalizer = request.body_connection.finalize().await?;
        (
            StatusCode::OK,
            [
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ],
            body.as_str(),
        )
            .write_to(finalizer, response_writer)
            .await
    }
}

/// GET /api/v1/fs/list?root={system|user}&path={path}
pub struct FsList;

impl picoserve::routing::MethodHandlerService<()> for FsList {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());
        let raw_path = crate::rest::query_val(query, "path")
            .unwrap_or_else(|| heapless::String::try_from("/").unwrap());

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let mut out = DirListBuffer::new();
        let is_system = root.as_str() == "system";

        let outcome: Result<Result<(), DirListError>, FsAccessError> = if is_system {
            with_system_fs_result(|fs| serialize_dir_entries(fs, clean_path.as_str(), &mut out))
        } else {
            with_user_fs_result(|fs| serialize_dir_entries(fs, clean_path.as_str(), &mut out))
        };

        let (code, body): (StatusCode, &str) = match outcome {
            Ok(Ok(())) => (StatusCode::OK, out.as_str()),
            Ok(Err(DirListError::Io)) => (StatusCode::NOT_FOUND, "{\"error\":\"directory_not_found\"}"),
            Ok(Err(DirListError::TooLarge)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "{\"error\":\"listing_too_large\"}")
            }
            Err(FsAccessError::Busy) => (StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_busy\"}"),
            Err(FsAccessError::NotMounted) => {
                (StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_not_mounted\"}")
            }
        };

        (
            code,
            [
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
            ],
            body,
        )
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

/// GET /api/v1/fs/read?root={system|user}&path={path}
pub struct FsRead;

impl picoserve::routing::MethodHandlerService<()> for FsRead {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "GET" && method != "HEAD" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());
        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let is_system = root.as_str() == "system";
        let mime = mime_for(clean_path.as_str());

        if is_system {
            let exists = match with_system_fs_result(|fs| {
                fs.metadata(&pbuf).map(|m| m.is_file()).unwrap_or(false)
            }) {
                Ok(v) => v,
                Err(FsAccessError::Busy) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        ("Content-Type", "application/json"),
                        "{\"error\":\"fs_busy\"}",
                    )
                        .write_to(request.body_connection.finalize().await?, response_writer)
                        .await;
                }
                Err(FsAccessError::NotMounted) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        ("Content-Type", "application/json"),
                        "{\"error\":\"fs_not_mounted\"}",
                    )
                        .write_to(request.body_connection.finalize().await?, response_writer)
                        .await;
                }
            };
            if !exists {
                return (
                    StatusCode::NOT_FOUND,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"file_not_found\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }

            let chunks = SystemFileChunks { path: pbuf, mime };
            let resp = ChunkedResponse::new(chunks).into_response();
            let resp = resp.with_header("Cache-Control", "no-cache, no-store, must-revalidate");
            return resp
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let exists = match with_user_fs_result(|fs| {
            fs.metadata(&pbuf).map(|m| m.is_file()).unwrap_or(false)
        }) {
            Ok(v) => v,
            Err(FsAccessError::Busy) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"fs_busy\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
            Err(FsAccessError::NotMounted) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"fs_not_mounted\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };
        if !exists {
            return (
                StatusCode::NOT_FOUND,
                ("Content-Type", "application/json"),
                "{\"error\":\"file_not_found\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let chunks = AppFileChunks { path: pbuf, mime };
        let resp = ChunkedResponse::new(chunks).into_response();
        let resp = resp.with_header("Cache-Control", "no-cache, no-store, must-revalidate");
        resp.write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

/// POST /api/v1/fs/write?root={root}&path={path}
/// Body contains raw data to write.
/// If root=system, strictly rejects with 403 Forbidden.
pub struct FsWrite;

impl picoserve::routing::MethodHandlerService<()> for FsWrite {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" && method != "PUT" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        // Strict partition boundary enforcement: system_fs is immutable ROM
        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        // Stage the full payload in a sibling temp file, then atomically rename
        // it over the target. The original file stays byte-for-byte intact until
        // every chunk has been persisted; a failed chunk never truncates it.
        let mut temp_path = heapless::String::<192>::new();
        {
            let path = clean_path.as_str();
            let (parent, name) = match path.rfind('/') {
                Some(idx) => (&path[..=idx], &path[idx + 1..]),
                None => ("/", path),
            };
            let _ = write!(temp_path, "{}.{}.tmp", parent, name);
        }
        let tmp_pbuf = match PathBuf::try_from(temp_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        // (status code, JSON body) of any failure that must abort the save.
        let mut failure: Option<(StatusCode, &'static str)> = None;

        match with_user_fs_result(|fs| {
            ensure_parent_dirs_user(fs, clean_path.as_str());
            fs.write(&tmp_pbuf, &[])
        }) {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                failure = Some((StatusCode::INTERNAL_SERVER_ERROR, "{\"error\":\"fs_init_write_failed\"}"));
            }
            Err(FsAccessError::Busy) => {
                failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_busy\"}"));
            }
            Err(FsAccessError::NotMounted) => {
                failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_not_mounted\"}"));
            }
        }

        let mut pos: u32 = 0;
        if failure.is_none() {
            // The default picoserve read_request timeout is 3s and stays active
            // for the whole handler. Streaming a large file through LittleFS in
            // small chunks exceeds it, so extend it for the body read. Use a
            // full 4096-byte block per write_chunk to minimize metadata commits
            // (each write_chunk reopens the file and commits).
            let mut reader = request
                .body_connection
                .body()
                .reader()
                .with_different_timeout(embassy_time::Duration::from_secs(120));
            let mut buf = [0u8; 4096];
            loop {
                let n = match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => {
                        failure = Some((StatusCode::INTERNAL_SERVER_ERROR, "{\"error\":\"fs_write_chunk_failed\"}"));
                        break;
                    }
                };

                match with_user_fs_result(|fs| {
                    fs.write_chunk(&tmp_pbuf, &buf[..n], OpenSeekFrom::Start(pos))
                }) {
                    Ok(Ok(())) => pos += n as u32,
                    Ok(Err(_)) => {
                        failure = Some((StatusCode::INTERNAL_SERVER_ERROR, "{\"error\":\"fs_write_chunk_failed\"}"));
                        break;
                    }
                    Err(FsAccessError::Busy) => {
                        failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_busy\"}"));
                        break;
                    }
                    Err(FsAccessError::NotMounted) => {
                        failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_not_mounted\"}"));
                        break;
                    }
                }
            }
        }

        if failure.is_none() {
            match with_user_fs_result(|fs| fs.rename(&tmp_pbuf, &pbuf)) {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    failure = Some((StatusCode::INTERNAL_SERVER_ERROR, "{\"error\":\"fs_rename_failed\"}"));
                }
                Err(FsAccessError::Busy) => {
                    failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_busy\"}"));
                }
                Err(FsAccessError::NotMounted) => {
                    failure = Some((StatusCode::SERVICE_UNAVAILABLE, "{\"error\":\"fs_not_mounted\"}"));
                }
            }
        }

        // On any error path the staged file must not linger.
        if failure.is_some() {
            let _ = with_user_fs_result(|fs| fs.remove(&tmp_pbuf));
        }

        let finalizer = request.body_connection.finalize().await?;
        if let Some((code, body)) = failure {
            return (code, ("Content-Type", "application/json"), body)
                .write_to(finalizer, response_writer)
                .await;
        }

        {
            let mut resp = heapless::String::<1024>::new();
            let encoded = write!(resp, r#"{{"status":"ok","bytes_written":{},"path":"#, pos)
                .and_then(|()| write_json_string(&mut resp, clean_path.as_str()))
                .and_then(|()| resp.write_str("}"));
            if encoded.is_err() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"response_too_large\"}",
                )
                    .write_to(finalizer, response_writer)
                    .await;
            }
            (
                StatusCode::OK,
                [
                    ("Content-Type", "application/json"),
                    ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ],
                resp.as_str(),
            )
                .write_to(finalizer, response_writer)
                .await
        }
    }
}

/// DELETE /api/v1/fs/entry?root={root}&path={path}
/// Also supports POST /api/v1/fs/entry?root={root}&path={path}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsEntry;

impl picoserve::routing::MethodHandlerService<()> for FsEntry {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "DELETE" && method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"cannot_delete_root\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = with_user_fs_result(|fs| {
            let is_dir = fs.metadata(&pbuf).map(|m| m.is_dir()).unwrap_or(false);
            if is_dir {
                fs.remove_dir_all(&pbuf)
            } else {
                fs.remove(&pbuf)
            }
        });

        let finalizer = request.body_connection.finalize().await?;
        match outcome {
            Ok(Ok(())) => {
                let mut resp = heapless::String::<1024>::new();
                let encoded = write!(resp, r#"{{"status":"deleted","path":""#)
                    .and_then(|()| write_json_string(&mut resp, clean_path.as_str()))
                    .and_then(|()| resp.write_str("}"));
                if encoded.is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ("Content-Type", "application/json"),
                        "{\"error\":\"response_too_large\"}",
                    )
                        .write_to(finalizer, response_writer)
                        .await;
                }
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Ok(Err(_)) => {
                (StatusCode::NOT_FOUND, ("Content-Type", "application/json"), "{\"error\":\"entry_not_found\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::Busy) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_busy\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::NotMounted) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}

/// POST /api/v1/fs/mkdir?root={root}&path={path}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsMkdir;

impl picoserve::routing::MethodHandlerService<()> for FsMkdir {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_path = match crate::rest::query_val(query, "path") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_path_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_path = match sanitize_fs_path(raw_path.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let pbuf = match PathBuf::try_from(clean_path.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"path_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = with_user_fs_result(|fs| fs.create_dir_all(&pbuf));
        let finalizer = request.body_connection.finalize().await?;

        match outcome {
            Ok(Ok(())) => {
                let mut resp = heapless::String::<1024>::new();
                let encoded = write!(resp, r#"{{"status":"created","path":""#)
                    .and_then(|()| write_json_string(&mut resp, clean_path.as_str()))
                    .and_then(|()| resp.write_str("}"));
                if encoded.is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ("Content-Type", "application/json"),
                        "{\"error\":\"response_too_large\"}",
                    )
                        .write_to(finalizer, response_writer)
                        .await;
                }
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Ok(Err(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ("Content-Type", "application/json"), "{\"error\":\"mkdir_failed\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::Busy) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_busy\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::NotMounted) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}

/// POST /api/v1/fs/rename?root={root}&src={src}&dst={dst}
/// Strictly rejects with 403 Forbidden if root=system.
pub struct FsRename;

impl picoserve::routing::MethodHandlerService<()> for FsRename {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if method != "POST" {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                ("Content-Type", "application/json"),
                "{\"error\":\"method_not_allowed\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let query = request.parts.query();
        let root = crate::rest::query_val(query, "root")
            .unwrap_or_else(|| heapless::String::try_from("user").unwrap());

        if root.as_str() == "system" {
            return (
                StatusCode::FORBIDDEN,
                ("Content-Type", "application/json"),
                "{\"error\":\"system_fs is read-only\"}",
            )
                .write_to(request.body_connection.finalize().await?, response_writer)
                .await;
        }

        let raw_src = match crate::rest::query_val(query, "src") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_src_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let raw_dst = match crate::rest::query_val(query, "dst") {
            Some(p) => p,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"missing_dst_param\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_src = match sanitize_fs_path(raw_src.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_src_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let clean_dst = match sanitize_fs_path(raw_dst.as_str()) {
            Some(p) if p.as_str() != "/" => p,
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"invalid_dst_path\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let src_pbuf = match PathBuf::try_from(clean_src.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"src_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let dst_pbuf = match PathBuf::try_from(clean_dst.as_str()) {
            Ok(p) => p,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    ("Content-Type", "application/json"),
                    "{\"error\":\"dst_too_long\"}",
                )
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        let outcome = with_user_fs_result(|fs| {
            ensure_parent_dirs_user(fs, clean_dst.as_str());
            fs.rename(&src_pbuf, &dst_pbuf)
        });

        let finalizer = request.body_connection.finalize().await?;
        match outcome {
            Ok(Ok(())) => {
                let mut resp = heapless::String::<1024>::new();
                let encoded = write!(resp, r#"{{"status":"renamed","src":""#)
                    .and_then(|()| write_json_string(&mut resp, clean_src.as_str()))
                    .and_then(|()| resp.write_str("\",\"dst\":\""))
                    .and_then(|()| write_json_string(&mut resp, clean_dst.as_str()))
                    .and_then(|()| resp.write_str("}"));
                if encoded.is_err() {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ("Content-Type", "application/json"),
                        "{\"error\":\"response_too_large\"}",
                    )
                        .write_to(finalizer, response_writer)
                        .await;
                }
                (StatusCode::OK, ("Content-Type", "application/json"), resp.as_str())
                    .write_to(finalizer, response_writer)
                    .await
            }
            Ok(Err(_)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, ("Content-Type", "application/json"), "{\"error\":\"rename_failed\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::Busy) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_busy\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
            Err(FsAccessError::NotMounted) => {
                (StatusCode::SERVICE_UNAVAILABLE, ("Content-Type", "application/json"), "{\"error\":\"fs_not_mounted\"}")
                    .write_to(finalizer, response_writer)
                    .await
            }
        }
    }
}
