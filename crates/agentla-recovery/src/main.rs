//! Synchronous recovery bootloader.
//!
//! Runs directly from flash 0x00000000 (XIP 0x10000000) as the primary bootloader.
//! On boot it looks for `/ota/update.bin` in USER_FS (0x00248000), validates
//! the `AGLA_FW` container, writes the payload into the single main app slot
//! (flash 0x00048000..0x00148000, XIP 0x10048000), deletes the candidate,
//! writes an ASCII status file, and jumps to the app. No executor, no USB,
//! no network, no heap.

#![no_std]
#![no_main]

use core::arch::asm;
use core::fmt::Write as _;

use agentla_boot_proto::FirmwareHeader;
use agentla_recovery::{
    align_up_4096, header_valid, parse_header, valid_vector, Crc32, APP_BASE_FLASH, APP_BASE_XIP,
    APP_SLOT_END_FLASH, HEADER_SIZE, STATUS_PATH, UPDATE_PATH, USER_BLOCK_COUNT, USER_FS_OFFSET,
};
use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;
use defmt_rtt as _;
use embassy_rp::flash::{Blocking, Flash, FLASH_BASE};
use embassy_rp::peripherals::FLASH;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::Peri;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::{Mutex, MutexGuard};
use embassy_time::Duration;
use littlefs2::driver::Storage;
use littlefs2::fs::{Allocation, File, Filesystem};
use littlefs2::io::{Error as IoError, Result as IoResult, SeekFrom};
use littlefs2::path::{Path, PathBuf};
use panic_halt as _;
use static_cell::StaticCell;

const FLASH_SIZE_BYTES: usize = 4 * 1024 * 1024;
const SECTOR_SIZE: usize = 4096;
const PAGE_SIZE: usize = 256;
const CHUNK_SIZE: usize = 512;

/// RP2350 Block Loop Image Definition.
/// Placed in the vector table's interrupt section so the RP2350 Bootrom
/// block-loop scanner finds it within the first 256 bytes of flash.
#[used]
#[link_section = ".vector_table.interrupts"]
static IMAGE_DEFINITION: [u32; 5] = [
    0xffff_ded3,
    0x1021_0142,
    0x0000_01ff,
    0x0000_0000,
    0xab12_3579,
];

type FlashHandle = Flash<'static, FLASH, Blocking, FLASH_SIZE_BYTES>;

// ---------------------------------------------------------------------------
// libc string shims for the littlefs2 C core.
//
// `littlefs2-sys` compiles `lfs.c` with its `lfs_util.h` helpers stripped, so
// the C core emits plain `strcpy`/`strchr`/`strspn`/`strcspn`/`strlen` calls.
// `compiler_builtins` supplies only the mem* family; we supply the str* family
// here (mirroring firmware-rs) because a no_std firmware links no libc.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
unsafe extern "C" fn strcpy(dst: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
    }
    dst
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *s.add(i);
            if b as core::ffi::c_int == c {
                return s.add(i) as *mut core::ffi::c_char;
            }
            if b == 0 {
                return core::ptr::null_mut();
            }
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(s: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strspn(s: *const core::ffi::c_char, accept: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            let mut found = false;
            let mut i = 0usize;
            while *accept.add(i) != 0 {
                if *accept.add(i) == *s.add(n) {
                    found = true;
                    break;
                }
                i += 1;
            }
            if !found {
                break;
            }
            n += 1;
        }
        n
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn strcspn(s: *const core::ffi::c_char, reject: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            let mut i = 0usize;
            while *reject.add(i) != 0 {
                if *reject.add(i) == *s.add(n) {
                    return n;
                }
                i += 1;
            }
            n += 1;
        }
        n
    }
}

/// The FLASH peripheral is a singleton. All littlefs partition I/O and all
/// direct app-slot programming go through this guard. The firmware is fully
/// synchronous (no executor, no interrupt handlers touch flash), so `try_lock`
/// can never fail here.
static FLASH_MUTEX: Mutex<CriticalSectionRawMutex, Option<FlashHandle>> = Mutex::new(None);

fn flash_guard() -> MutexGuard<'static, CriticalSectionRawMutex, Option<FlashHandle>> {
    FLASH_MUTEX
        .try_lock()
        .expect("FLASH mutex unavailable in boot path")
}

// ---------------------------------------------------------------------------
// LittleFS storage driver bounded strictly to USER_FS.
// ---------------------------------------------------------------------------

struct UserFlashStorage {
    offset: u32,
}

impl Storage for UserFlashStorage {
    const READ_SIZE: usize = 16;
    const WRITE_SIZE: usize = PAGE_SIZE;
    const BLOCK_SIZE: usize = SECTOR_SIZE;
    const BLOCK_COUNT: usize = USER_BLOCK_COUNT;
    const BLOCK_CYCLES: isize = 500;

    type CACHE_SIZE = littlefs2::consts::U256;
    type LOOKAHEAD_SIZE = littlefs2::consts::U64;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> IoResult<usize> {
        let abs = self.offset + off as u32;
        let limit = self.offset + (USER_BLOCK_COUNT * SECTOR_SIZE) as u32;
        if abs + buf.len() as u32 > limit {
            return Err(IoError::IO);
        }
        let ptr = (FLASH_BASE as usize + abs as usize) as *const u8;
        unsafe { core::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), buf.len()) };
        Ok(buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> IoResult<usize> {
        let abs = self.offset + off as u32;
        let limit = self.offset + (USER_BLOCK_COUNT * SECTOR_SIZE) as u32;
        if abs + data.len() as u32 > limit {
            return Err(IoError::IO);
        }
        let mut guard = flash_guard();
        let flash = guard.as_mut().ok_or(IoError::IO)?;
        flash.blocking_write(abs, data).map_err(|_| IoError::IO)?;
        Ok(data.len())
    }

    fn erase(&mut self, off: usize, len: usize) -> IoResult<usize> {
        let from = self.offset + off as u32;
        let to = from + len as u32;
        let limit = self.offset + (USER_BLOCK_COUNT * SECTOR_SIZE) as u32;
        if to > limit {
            return Err(IoError::IO);
        }
        let mut guard = flash_guard();
        let flash = guard.as_mut().ok_or(IoError::IO)?;
        flash.blocking_erase(from, to).map_err(|_| IoError::IO)?;
        Ok(len)
    }
}

// ---------------------------------------------------------------------------
// App-slot primitives. Bounds enforced so OTA can never reach Katapult
// (< 0x00048000), or SYSTEM_FS (>= 0x00148000).
// ---------------------------------------------------------------------------

fn flash_slot_erase(from: u32, to: u32) -> Result<(), ()> {
    if from < APP_BASE_FLASH
        || to > APP_SLOT_END_FLASH
        || from >= to
        || from % SECTOR_SIZE as u32 != 0
        || to % SECTOR_SIZE as u32 != 0
    {
        return Err(());
    }
    let mut guard = flash_guard();
    let flash = guard.as_mut().ok_or(())?;
    flash.blocking_erase(from, to).map_err(|_| ())
}

fn flash_slot_write(offset: u32, data: &[u8]) -> Result<(), ()> {
    let end = offset + data.len() as u32;
    if data.is_empty()
        || offset < APP_BASE_FLASH
        || end > APP_SLOT_END_FLASH
        || offset % PAGE_SIZE as u32 != 0
    {
        return Err(());
    }
    let mut guard = flash_guard();
    let flash = guard.as_mut().ok_or(())?;
    for (page, chunk) in data.chunks(PAGE_SIZE).enumerate() {
        flash
            .blocking_write(offset + (page * PAGE_SIZE) as u32, chunk)
            .map_err(|_| ())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Outcome / status reporting.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum OtaOutcome {
    Ok { size: u32, crc: u32 },
    Error(&'static str),
    MountFail,
}

/// Tiny stack-only `fmt::Write` sink: no heap, fixed 48-byte status buffer.
struct StackWriter<'a> {
    buf: &'a mut [u8],
    len: usize,
}

impl core::fmt::Write for StackWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(core::fmt::Error)?;
        if end > self.buf.len() {
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

fn write_status(fs: &Filesystem<UserFlashStorage>, outcome: OtaOutcome) {
    let Ok(path) = PathBuf::try_from(STATUS_PATH) else {
        return;
    };
    let mut buf = [0u8; 48];
    let mut writer = StackWriter {
        buf: &mut buf,
        len: 0,
    };
    let written = match outcome {
        OtaOutcome::Ok { size, crc } => write!(writer, "ok:{}:{:08X}", size, crc),
        OtaOutcome::Error(code) => write!(writer, "error:{}", code),
        OtaOutcome::MountFail => return,
    };
    if written.is_ok() {
        let _ = fs.write(&path, &writer.buf[..writer.len]);
    }
}

// ---------------------------------------------------------------------------
// Header verification and streaming flash update.
// ---------------------------------------------------------------------------

struct ImageMeta {
    header: FirmwareHeader,
    sp: u32,
    pc: u32,
}

/// Compute CRC32 over `size` bytes of the currently installed image via XIP.
fn installed_crc(size: usize) -> u32 {
    let mut crc = Crc32::new();
    let base = APP_BASE_XIP as *const u8;
    let mut block = [0u8; CHUNK_SIZE];
    let mut off = 0usize;
    while off < size {
        let n = core::cmp::min(size - off, block.len());
        unsafe { core::ptr::copy_nonoverlapping(base.add(off), block.as_mut_ptr(), n) };
        crc.update(&block[..n]);
        off += n;
    }
    crc.finish()
}

fn read_image_meta(fs: &Filesystem<UserFlashStorage>, path: &Path) -> IoResult<ImageMeta> {
    File::open_and_then(fs, path, |file| {
        let mut raw = [0u8; HEADER_SIZE];
        if file.read(&mut raw)? != HEADER_SIZE {
            return Err(IoError::IO);
        }
        let header = parse_header(&raw).ok_or(IoError::IO)?;

        // The first eight payload bytes are the initial SP/PC vector.
        let mut vec = [0u8; 8];
        if file.read(&mut vec)? != 8 {
            return Err(IoError::IO);
        }
        let sp = u32::from_le_bytes(vec[..4].try_into().unwrap());
        let pc = u32::from_le_bytes(vec[4..].try_into().unwrap());
        Ok(ImageMeta { header, sp, pc })
    })
}

fn perform_update(fs: &Filesystem<UserFlashStorage>, path: &Path) -> OtaOutcome {
    if !fs.exists(path) {
        return OtaOutcome::Error("no_update");
    }
    let meta = match read_image_meta(fs, path) {
        Ok(m) => m,
        Err(_) => return OtaOutcome::Error("bad_file"),
    };

    if !header_valid(&meta.header) {
        return OtaOutcome::Error("bad_header");
    }
    if !valid_vector(meta.sp, meta.pc) {
        return OtaOutcome::Error("bad_vector");
    }

    let size = meta.header.payload_size as usize;
    let expected_crc = meta.header.payload_crc;

    // Idempotency: if the installed image already matches, do not erase/write.
    if installed_crc(size) == expected_crc {
        defmt::info!("image already matches expected CRC; skipping write");
        return OtaOutcome::Ok {
            size: meta.header.payload_size,
            crc: expected_crc,
        };
    }

    // Erase exactly the sectors covering the declared payload.
    let erase_end = APP_BASE_FLASH + align_up_4096(size) as u32;
    if flash_slot_erase(APP_BASE_FLASH, erase_end).is_err() {
        return OtaOutcome::Error("erase_failed");
    }

    // Stream the payload from the file into the app slot, 256-byte pages.
    let stream = File::open_and_then(fs, path, |file| -> IoResult<(u32, u32)> {
        file.seek(SeekFrom::Start(HEADER_SIZE as u32))?;
        let mut crc = Crc32::new();
        let mut chunk = [0u8; CHUNK_SIZE];
        let mut page = [0xFFu8; PAGE_SIZE];
        let mut page_len = 0usize;
        let mut written = 0u32;

        loop {
            let n = file.read(&mut chunk)?;
            if n == 0 {
                break;
            }
            if written as usize + n > size {
                return Err(IoError::IO); // payload longer than declared
            }
            crc.update(&chunk[..n]);
            for &b in &chunk[..n] {
                page[page_len] = b;
                page_len += 1;
                written += 1;
                if page_len == PAGE_SIZE {
                    let off = APP_BASE_FLASH + written - PAGE_SIZE as u32;
                    flash_slot_write(off, &page).map_err(|_| IoError::IO)?;
                    page.fill(0xFF);
                    page_len = 0;
                }
            }
        }
        if page_len > 0 {
            let off = APP_BASE_FLASH + written - page_len as u32;
            flash_slot_write(off, &page[..page_len]).map_err(|_| IoError::IO)?;
        }
        Ok((written, crc.finish()))
    });

    match stream {
        Ok((written, crc)) if written == meta.header.payload_size && crc == expected_crc => {
            OtaOutcome::Ok {
                size: written,
                crc,
            }
        }
        Ok((written, crc)) => {
            defmt::warn!(
                "stream mismatch: written={} crc=0x{:08X} expected=0x{:08X}",
                written,
                crc,
                expected_crc
            );
            OtaOutcome::Error("crc_mismatch")
        }
        Err(_) => OtaOutcome::Error("stream_failed"),
    }
}

/// Mount USER_FS exactly once (never format/wipe), process the candidate,
/// quarantine it, and record a status string.
fn run_ota() -> OtaOutcome {
    static USER_STORAGE: StaticCell<UserFlashStorage> = StaticCell::new();
    static USER_ALLOC: StaticCell<Allocation<UserFlashStorage>> = StaticCell::new();

    let storage = USER_STORAGE.init(UserFlashStorage {
        offset: USER_FS_OFFSET,
    });
    let alloc = USER_ALLOC.init(Allocation::new());

    let fs = match Filesystem::mount(alloc, storage) {
        Ok(fs) => fs,
        Err(_) => {
            defmt::warn!("USER_FS mount failed; skipping OTA (no format attempted)");
            return OtaOutcome::MountFail;
        }
    };

    let Ok(path) = PathBuf::try_from(UPDATE_PATH) else {
        return OtaOutcome::MountFail;
    };
    let outcome = perform_update(&fs, &path);

    // Quarantine the candidate on both success and failure so it is not
    // re-applied on every boot.
    let _ = fs.remove(&path);
    write_status(&fs, outcome);
    outcome
}

// ---------------------------------------------------------------------------
// Hand-off to the main application.
// ---------------------------------------------------------------------------

fn jump_to_app(watchdog: Peri<'static, embassy_rp::peripherals::WATCHDOG>) -> ! {
    // Ensure prior flash writes are visible through XIP.
    unsafe { asm!("dsb sy", "isb sy") };

    let sp = unsafe { core::ptr::read_volatile(APP_BASE_XIP as *const u32) };
    let pc = unsafe { core::ptr::read_volatile((APP_BASE_XIP + 4) as *const u32) };

    if !valid_vector(sp, pc) {
        defmt::error!(
            "no valid application image at 0x10048000 (sp=0x{:08X}, pc=0x{:08X}); resetting",
            sp,
            pc
        );
        let mut wd = Watchdog::new(watchdog);
        wd.start(Duration::from_millis(1500));
        loop {
            unsafe { asm!("wfi") };
        }
    }

    unsafe {
        // Mask interrupts while we reprogram VTOR/MSP so a pending IRQ cannot
        // fire with a half-switched vector table.
        asm!("cpsid i");
        // Clear all pending IRQs so nothing fires the instant we unmask.
        let nvic = &*cortex_m::peripheral::NVIC::PTR;
        for reg in nvic.icpr.iter() {
            reg.write(0xFFFF_FFFF);
        }
        (*SCB::PTR).vtor.write(APP_BASE_XIP);
        asm!("dsb sy", "isb sy");
        // Load the app stack pointer first, then unmask interrupts, then branch.
        // The application's startup expects PRIMASK clear on entry — leaving it
        // set leaves USB/timers/executor dead (observed as no USB enumeration).
        asm!(
            "msr msp, {}",
            "cpsie i",
            "dsb sy",
            "isb sy",
            "bx {}",
            in(reg) sp,
            in(reg) pc,
            options(noreturn)
        );
    }
}

#[entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());
    defmt::info!("AgentLA recovery bootloader start (XIP 0x10000000)");

    {
        let mut guard = FLASH_MUTEX.try_lock().unwrap();
        *guard = Some(Flash::<_, Blocking, FLASH_SIZE_BYTES>::new_blocking(p.FLASH));
    }

    match run_ota() {
        OtaOutcome::Ok { size, crc } => {
            defmt::info!("OTA applied: {} bytes, crc=0x{:08X}", size, crc)
        }
        OtaOutcome::Error(code) => defmt::warn!("OTA not applied: {}", code),
        OtaOutcome::MountFail => defmt::warn!("OTA skipped: USER_FS unavailable"),
    }

    // Primary bootloader entry: hand off to the single app slot.
    jump_to_app(p.WATCHDOG)
}
