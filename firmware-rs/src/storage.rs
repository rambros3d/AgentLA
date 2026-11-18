use embassy_rp::flash::{Blocking, Flash, FLASH_BASE};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use littlefs2::driver::Storage;
use littlefs2::fs::{Allocation, Filesystem};
use static_cell::StaticCell;

pub const BLOCK_SIZE: usize = 4096;

#[cfg(feature = "board-bakuprobe")]
pub const FLASH_SIZE_BYTES: usize = 16 * 1024 * 1024;
#[cfg(not(feature = "board-bakuprobe"))]
pub const FLASH_SIZE_BYTES: usize = 4 * 1024 * 1024;

/// Katapult stage-0: flash 0x00000000..0x00008000, 32 KB (XIP 0x10000000).
/// It jumps to the recovery application at XIP 0x10008000. Never written by
/// this firmware.
pub const BOOTLOADER_OFFSET: u32 = 0x00000000;
pub const BOOTLOADER_SIZE_BYTES: usize = 32 * 1024;

/// Recovery application: flash 0x00008000..0x00048000, 256 KB
/// (XIP 0x10008000). Built by the agentla-recovery crate and shipped in the
/// UF2 alongside stage-0; never written by main-firmware OTA.
pub const RECOVERY_OFFSET: u32 = 0x00008000;
pub const RECOVERY_SIZE_BYTES: usize = 256 * 1024;

/// Main application slot: flash 0x00048000..0x00148000 (XIP 0x10048000, 1 MB).
/// Firmware is linked at 0x10048000. Nothing below APP_SLOT_OFFSET (stage-0 +
/// recovery) or at/above APP_SLOT_END (filesystems) may ever be erased or
/// programmed by an OTA operation.
pub const APP_SLOT_OFFSET: u32 = 0x00048000;
pub const APP_SLOT_SIZE: usize = 1024 * 1024; // 0x100000 bytes
pub const APP_SLOT_END: u32 = 0x00148000;

/// SYSTEM_FS partition: 1 MB (256 sectors of 4 KB), flash 0x00148000..0x00248000.
pub const SYSTEM_FS_OFFSET: u32 = 0x00148000;
pub const SYSTEM_FS_SIZE_BYTES: usize = 1024 * 1024;
pub const SYSTEM_BLOCK_COUNT: usize = SYSTEM_FS_SIZE_BYTES / BLOCK_SIZE; // 256 blocks
pub const SYSTEM_FS_END: u32 = 0x00248000;

/// USER_FS partition: 1760 KB (440 sectors of 4 KB), flash 0x00248000..0x00400000.
/// Staged OTA images are written here as LittleFS files (e.g. /ota/update.bin),
/// never via direct flash writes from the OTA endpoints.
pub const USER_FS_OFFSET: u32 = 0x00248000;
pub const USER_FS_END: u32 = 0x00400000;
pub const USER_FS_SIZE_BYTES: usize = (USER_FS_END - USER_FS_OFFSET) as usize;
pub const USER_BLOCK_COUNT: usize = USER_FS_SIZE_BYTES / BLOCK_SIZE; // 440 blocks

/// The `FLASH` peripheral is a single instance: the physical flash object lives in this
/// mutex from [`init_filesystem`] on, shared by LittleFS storage instances and the
/// profile-window primitives.
static FLASH_MUTEX: Mutex<CriticalSectionRawMutex, Option<FlashHandle>> = Mutex::new(None);

pub type FlashHandle = Flash<'static, FLASH, Blocking, FLASH_SIZE_BYTES>;

/// LittleFS storage driver bounded to a fixed sector slice.
pub struct FlashStorageGeneric<const BLOCKS: usize> {
    offset: u32,
}

pub type SystemFlashStorage = FlashStorageGeneric<SYSTEM_BLOCK_COUNT>;
pub type UserFlashStorage = FlashStorageGeneric<USER_BLOCK_COUNT>;
pub type FlashStorage = UserFlashStorage;

fn flash_guard() -> embassy_sync::mutex::MutexGuard<'static, CriticalSectionRawMutex, Option<FlashHandle>> {
    let mut guard = FLASH_MUTEX
        .try_lock()
        .expect("FLASH mutex unavailable (init_filesystem not called or nested op)");
    guard
        .as_mut()
        .expect("FLASH peripheral not initialized at init_filesystem");
    guard
}

impl<const BLOCKS: usize> Storage for FlashStorageGeneric<BLOCKS> {
    const READ_SIZE: usize = 16;
    const WRITE_SIZE: usize = 256;
    const BLOCK_SIZE: usize = 4096;
    const BLOCK_COUNT: usize = BLOCKS;
    const BLOCK_CYCLES: isize = 500;

    type CACHE_SIZE = littlefs2::consts::U256;
    type LOOKAHEAD_SIZE = littlefs2::consts::U64;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> littlefs2::io::Result<usize> {
        let abs_off = self.offset + off as u32;
        let limit = self.offset + (BLOCKS * BLOCK_SIZE) as u32;
        debug_assert!(
            abs_off >= self.offset && (abs_off + buf.len() as u32) <= limit,
            "littlefs read out of partition bounds"
        );
        let flash_ptr = (FLASH_BASE as usize + abs_off as usize) as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
        let abs_off = self.offset + off as u32;
        let limit = self.offset + (BLOCKS * BLOCK_SIZE) as u32;
        debug_assert!(
            abs_off >= self.offset && (abs_off + data.len() as u32) <= limit,
            "littlefs write out of partition bounds"
        );
        // Propagate the error. A blocking flash write can fail (XIP contention,
        // write-protected sector); unwrapping turned that into a panic in the
        // boot path, which with no watchdog running left the probe alive but
        // with no HTTP server.
        flash_guard()
            .as_mut()
            .ok_or(littlefs2::io::Error::IO)?
            .blocking_write(abs_off, data)
            .map_err(|_| littlefs2::io::Error::IO)?;
        Ok(data.len())
    }

    fn erase(&mut self, off: usize, len: usize) -> littlefs2::io::Result<usize> {
        let from = self.offset + off as u32;
        let to = from + len as u32;
        let limit = self.offset + (BLOCKS * BLOCK_SIZE) as u32;
        debug_assert!(
            from >= self.offset && to <= limit,
            "littlefs erase out of partition bounds"
        );
        flash_guard()
            .as_mut()
            .ok_or(littlefs2::io::Error::IO)?
            .blocking_erase(from, to)
            .map_err(|_| littlefs2::io::Error::IO)?;
        Ok(len)
    }
}

pub struct SafeFilesystem<S: Storage + 'static>(pub Filesystem<'static, S>);
unsafe impl<S: Storage + 'static> Send for SafeFilesystem<S> {}
unsafe impl<S: Storage + 'static> Sync for SafeFilesystem<S> {}

impl<S: Storage + 'static> SafeFilesystem<S> {
    pub fn filesystem(&self) -> &Filesystem<'static, S> {
        &self.0
    }

    pub unsafe fn filesystem_mut(&mut self) -> &mut Filesystem<'static, S> {
        &mut self.0
    }
}

pub static SYSTEM_FS_MUTEX: Mutex<CriticalSectionRawMutex, Option<SafeFilesystem<SystemFlashStorage>>> = Mutex::new(None);
pub static USER_FS_MUTEX: Mutex<CriticalSectionRawMutex, Option<SafeFilesystem<UserFlashStorage>>> = Mutex::new(None);

pub static SYSTEM_FS_WAS_FORMATTED: portable_atomic::AtomicBool = portable_atomic::AtomicBool::new(false);
pub static USER_FS_WAS_FORMATTED: portable_atomic::AtomicBool = portable_atomic::AtomicBool::new(false);

/// Backward compatibility: `FS_WAS_FORMATTED` monitors the User partition format status.
pub static FS_WAS_FORMATTED: &portable_atomic::AtomicBool = &USER_FS_WAS_FORMATTED;

/// Why a filesystem could not be accessed by an FS REST endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsAccessError {
    /// The partition mutex is held by another task; retry later.
    Busy,
    /// The partition is not mounted.
    NotMounted,
}

/// Like [`with_system_fs`] but distinguishes a contended mutex from an
/// unmounted partition so callers can return `fs_busy` instead of misreporting
/// `fs_not_mounted`.
pub fn with_system_fs_result<R>(
    f: impl FnOnce(&Filesystem<'static, SystemFlashStorage>) -> R,
) -> Result<R, FsAccessError> {
    let guard = SYSTEM_FS_MUTEX.try_lock().map_err(|_| FsAccessError::Busy)?;
    let fs = guard.as_ref().ok_or(FsAccessError::NotMounted)?.filesystem();
    Ok(f(fs))
}

/// Like [`with_user_fs`] but distinguishes a contended mutex from an unmounted
/// partition.
pub fn with_user_fs_result<R>(
    f: impl FnOnce(&Filesystem<'static, UserFlashStorage>) -> R,
) -> Result<R, FsAccessError> {
    let guard = USER_FS_MUTEX.try_lock().map_err(|_| FsAccessError::Busy)?;
    let fs = guard.as_ref().ok_or(FsAccessError::NotMounted)?.filesystem();
    Ok(f(fs))
}

/// Non-blocking probe used by `/api/v1/fs/stat`, whose usage helpers keep a
/// cached value on an unmounted partition but must still surface contention.
pub fn system_fs_busy() -> bool {
    SYSTEM_FS_MUTEX.try_lock().is_err()
}

/// Non-blocking probe used by `/api/v1/fs/stat`; see [`system_fs_busy`].
pub fn user_fs_busy() -> bool {
    USER_FS_MUTEX.try_lock().is_err()
}

/// Run `f` with the mounted SYSTEM_FS (WebUI partition), if available.
pub fn with_system_fs<R>(f: impl FnOnce(&Filesystem<'static, SystemFlashStorage>) -> R) -> Option<R> {
    with_system_fs_result(f).ok()
}

/// Run `f` with the mounted USER_FS (user apps and data partition), if available.
pub fn with_user_fs<R>(f: impl FnOnce(&Filesystem<'static, UserFlashStorage>) -> R) -> Option<R> {
    with_user_fs_result(f).ok()
}

static SYSTEM_USAGE_CACHE: portable_atomic::AtomicUsize = portable_atomic::AtomicUsize::new(155_599);
static USER_USAGE_CACHE: portable_atomic::AtomicUsize = portable_atomic::AtomicUsize::new(8_192);

pub fn system_fs_usage() -> (usize, usize) {
    let total = SYSTEM_FS_SIZE_BYTES;
    if let Some(used) = with_system_fs(|fs| {
        let avail = fs.available_space().unwrap_or(total);
        total.saturating_sub(avail)
    }) {
        SYSTEM_USAGE_CACHE.store(used, portable_atomic::Ordering::Relaxed);
        (used, total)
    } else {
        (SYSTEM_USAGE_CACHE.load(portable_atomic::Ordering::Relaxed), total)
    }
}

pub fn user_fs_usage() -> (usize, usize) {
    let total = USER_FS_SIZE_BYTES;
    if let Some(used) = with_user_fs(|fs| {
        let avail = fs.available_space().unwrap_or(total);
        total.saturating_sub(avail)
    }) {
        USER_USAGE_CACHE.store(used, portable_atomic::Ordering::Relaxed);
        (used, total)
    } else {
        (USER_USAGE_CACHE.load(portable_atomic::Ordering::Relaxed), total)
    }
}

pub fn erase_and_format_partition(part: &str) -> Result<(), ()> {
    match part {
        "system" => {
            let mut guard = flash_guard();
            let flash = guard.as_mut().unwrap();
            flash.blocking_erase(SYSTEM_FS_OFFSET, SYSTEM_FS_END).map_err(|_| ())?;
            SYSTEM_FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
        }
        "user" => {
            let mut guard = flash_guard();
            let flash = guard.as_mut().unwrap();
            flash.blocking_erase(USER_FS_OFFSET, USER_FS_END).map_err(|_| ())?;
            USER_FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
        }
        _ => return Err(()),
    }
    Ok(())
}

/// Backward compatibility alias: `with_fs` routes to `with_user_fs`.
pub fn with_fs<R>(f: impl FnOnce(&Filesystem<'static, UserFlashStorage>) -> R) -> Option<R> {
    with_user_fs(f)
}

/// Safe format protection logic: Format ONLY the user partition.
/// `SYSTEM_FS` (System WebUI) is strictly isolated and can NEVER be reformatted
/// or erased by user partition format operations.
pub fn format_user_fs() -> Result<(), littlefs2::io::Error> {
    let mut guard = USER_FS_MUTEX.try_lock().map_err(|_| littlefs2::io::Error::IO)?;
    let safe_fs = guard.as_mut().ok_or(littlefs2::io::Error::IO)?;
    defmt::warn!("Reformatting USER_FS partition (SYSTEM_FS protected)...");
    USER_FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
    // UserFlashStorage bounds are USER_FS_OFFSET..USER_FS_END,
    // SYSTEM_FS remains completely untouched.
    let _ = safe_fs;
    Ok(())
}

pub fn init_filesystem(flash: FlashHandle) -> bool {
    // Take permanent ownership of the FLASH peripheral
    {
        let mut guard = FLASH_MUTEX.try_lock().expect("flash mutex free at boot");
        *guard = Some(flash);
    }

    // 1. Mount SYSTEM_FS (1 MB at 0x00148000)
    static SYS_ALLOC: StaticCell<Allocation<SystemFlashStorage>> = StaticCell::new();
    static SYS_STORAGE: StaticCell<SystemFlashStorage> = StaticCell::new();

    let sys_storage = SYS_STORAGE.init(SystemFlashStorage {
        offset: SYSTEM_FS_OFFSET,
    });
    let sys_alloc = SYS_ALLOC.init(Allocation::new());

    defmt::info!(
        "Mounting SYSTEM_FS partition ({} KiB at offset 0x{:08x})...",
        SYSTEM_FS_SIZE_BYTES / 1024,
        SYSTEM_FS_OFFSET
    );

    let sys_mounted = Filesystem::mount_or_else(sys_alloc, sys_storage, |_err, st, _al| {
        defmt::warn!("Formatting uninitialized SYSTEM_FS partition...");
        SYSTEM_FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
        Filesystem::format(st)
    });

    match sys_mounted {
        Ok(fs) => {
            defmt::info!(
                "SYSTEM_FS mounted at offset 0x{:08x} (formatted_on_boot={})",
                SYSTEM_FS_OFFSET,
                SYSTEM_FS_WAS_FORMATTED.load(portable_atomic::Ordering::Relaxed)
            );
            let mut guard = SYSTEM_FS_MUTEX.try_lock().unwrap();
            *guard = Some(SafeFilesystem(fs));
        }
        Err(e) => {
            defmt::error!(
                "Failed to mount SYSTEM_FS partition: {:?}",
                defmt::Debug2Format(&e)
            );
        }
    }

    // 2. Mount USER_FS (1760 KB at 0x00248000)
    static USER_ALLOC: StaticCell<Allocation<UserFlashStorage>> = StaticCell::new();
    static USER_STORAGE: StaticCell<UserFlashStorage> = StaticCell::new();

    let user_storage = USER_STORAGE.init(UserFlashStorage {
        offset: USER_FS_OFFSET,
    });
    let user_alloc = USER_ALLOC.init(Allocation::new());

    defmt::info!(
        "Mounting USER_FS partition ({} KiB at offset 0x{:08x})...",
        USER_FS_SIZE_BYTES / 1024,
        USER_FS_OFFSET
    );

    let user_mounted = Filesystem::mount_or_else(user_alloc, user_storage, |_err, st, _al| {
        defmt::warn!("Formatting uninitialized USER_FS partition (may take a second)...");
        USER_FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
        Filesystem::format(st)
    });

    match user_mounted {
        Ok(fs) => {
            defmt::info!(
                "USER_FS mounted at offset 0x{:08x} (formatted_on_boot={})",
                USER_FS_OFFSET,
                USER_FS_WAS_FORMATTED.load(portable_atomic::Ordering::Relaxed)
            );
            let mut guard = USER_FS_MUTEX.try_lock().unwrap();
            *guard = Some(SafeFilesystem(fs));
            drop(guard);
            crate::apps::seed_default_apps();
            true
        }
        Err(e) => {
            defmt::error!(
                "Failed to mount USER_FS partition: {:?}",
                defmt::Debug2Format(&e)
            );
            false
        }
    }
}

// ---------------------------------------------------------------------------
// OTA primitives. The only direct-flash caller is the legacy/alternate HTTP
// stream handler in `crate::rest`. Bounds are enforced here so an OTA can never
// reach the Katapult stage-0/recovery images below 0x48000 or the filesystems
// at/above 0x148000. The preferred update flow stages the image as a USER_FS
// file (/ota/update.bin) and lets the recovery app apply it.
// ---------------------------------------------------------------------------

/// Erases a 4 KB sector range inside the main application slot [from, to).
pub fn flash_slot_erase(from: u32, to: u32) -> Result<(), ()> {
    if from < APP_SLOT_OFFSET
        || to > APP_SLOT_END
        || from >= to
        || from % BLOCK_SIZE as u32 != 0
        || to % BLOCK_SIZE as u32 != 0
    {
        return Err(());
    }

    flash_guard()
        .as_mut()
        .unwrap()
        .blocking_erase(from, to)
        .map_err(|_| ())
}

/// Writes a page-aligned buffer into the main application slot.
pub fn flash_slot_write(offset: u32, data: &[u8]) -> Result<(), ()> {
    let to = offset + data.len() as u32;
    if offset < APP_SLOT_OFFSET || to > APP_SLOT_END || data.is_empty() {
        return Err(());
    }

    let mut guard = flash_guard();
    let flash = guard.as_mut().unwrap();
    for (page, chunk) in data.chunks(256).enumerate() {
        flash
            .blocking_write(offset + (page * 256) as u32, chunk)
            .map_err(|_| ())?;
    }
    Ok(())
}
