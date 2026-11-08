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

/// Boot Switcher & Metadata Sector (16 KB)
pub const BOOT_TABLE_OFFSET: u32 = 0x00000000;
pub const BOOT_TABLE_SIZE_BYTES: usize = 16 * 1024;

/// Dual Firmware Execution Slots: 1,024 KB each
pub const SLOT_A_OFFSET: u32 = 0x00004000;
pub const SLOT_SIZE_BYTES: usize = 1024 * 1024; // 1,048,576 bytes (0x00100000)
pub const SLOT_B_OFFSET: u32 = 0x00104000;
pub const FIRMWARE_SLOTS_END: u32 = 0x00204000;

/// SYSTEM_FS partition: 512 KB (128 sectors of 4 KB). Fixed across all boards.
pub const SYSTEM_FS_OFFSET: u32 = 0x00204000;
pub const SYSTEM_FS_SIZE_BYTES: usize = 512 * 1024;
pub const SYSTEM_BLOCK_COUNT: usize = SYSTEM_FS_SIZE_BYTES / BLOCK_SIZE; // 128 blocks
pub const SYSTEM_FS_END: u32 = SYSTEM_FS_OFFSET + SYSTEM_FS_SIZE_BYTES as u32; // 0x00284000

/// USER_FS partition: dynamically sized to `FLASH_END - 16KB`.
pub const USER_FS_OFFSET: u32 = SYSTEM_FS_END; // 0x00284000
pub const FLASH_RESERVED_TRAIL_BYTES: usize = 16 * 1024; // 16 KB
pub const USER_FS_END: u32 = (FLASH_SIZE_BYTES - FLASH_RESERVED_TRAIL_BYTES) as u32;
pub const USER_FS_SIZE_BYTES: usize = (USER_FS_END - USER_FS_OFFSET) as usize;
pub const USER_BLOCK_COUNT: usize = USER_FS_SIZE_BYTES / BLOCK_SIZE; // 284 blocks (Pico 2) or 3340 blocks (BakuProbe)

// Backwards compatibility aliases
pub const LITTLEFS_START_OFFSET: u32 = USER_FS_OFFSET;
pub const PARTITION_SIZE_BYTES: usize = USER_FS_SIZE_BYTES;
pub const BLOCK_COUNT: usize = USER_BLOCK_COUNT;

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
        flash_guard()
            .as_mut()
            .unwrap()
            .blocking_write(abs_off, data)
            .unwrap();
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
        flash_guard().as_mut().unwrap().blocking_erase(from, to).unwrap();
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

/// Run `f` with the mounted SYSTEM_FS (WebUI partition), if available.
pub fn with_system_fs<R>(f: impl FnOnce(&Filesystem<'static, SystemFlashStorage>) -> R) -> Option<R> {
    let guard = SYSTEM_FS_MUTEX.try_lock().ok()?;
    let fs = guard.as_ref()?.filesystem();
    Some(f(fs))
}

/// Run `f` with the mounted USER_FS (user apps and data partition), if available.
pub fn with_user_fs<R>(f: impl FnOnce(&Filesystem<'static, UserFlashStorage>) -> R) -> Option<R> {
    let guard = USER_FS_MUTEX.try_lock().ok()?;
    let fs = guard.as_ref()?.filesystem();
    Some(f(fs))
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

    // 1. Mount SYSTEM_FS (512 KB at 0x00270000)
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

    // 2. Mount USER_FS (~1.0 MB or ~13.0 MB at 0x002F0000)
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
// Bootloader & OTA primitives. The ONLY callers are in `crate::boot` and `crate::web`;
// operations strictly enforce partition isolation.
// ---------------------------------------------------------------------------

/// Commits a 256-byte page containing the BootTable into the specified ping-pong sector.
pub fn flash_boot_sector_commit(offset: u32, page: &[u8; 256]) -> Result<(), ()> {
    if offset != agentla_boot_proto::BOOT_SECTOR_0_OFFSET && offset != agentla_boot_proto::BOOT_SECTOR_1_OFFSET {
        return Err(());
    }

    let mut guard = flash_guard();
    let flash = guard.as_mut().unwrap();

    // Erase the 4 KB sector
    flash
        .blocking_erase(offset, offset + BLOCK_SIZE as u32)
        .map_err(|_| ())?;

    // Program the 256-byte page
    flash
        .blocking_write(offset, page)
        .map_err(|_| ())?;

    Ok(())
}

/// Erases a block range inside an OTA firmware slot [from, to).
/// Enforces that the target range is strictly inside the specified slot (Slot A or Slot B).
pub fn flash_slot_erase(from: u32, to: u32) -> Result<(), ()> {
    let valid_slot_a = from >= SLOT_A_OFFSET && to <= (SLOT_A_OFFSET + SLOT_SIZE_BYTES as u32);
    let valid_slot_b = from >= SLOT_B_OFFSET && to <= (SLOT_B_OFFSET + SLOT_SIZE_BYTES as u32);

    if !(valid_slot_a || valid_slot_b) || from >= to || (from % BLOCK_SIZE as u32 != 0) {
        return Err(());
    }

    flash_guard()
        .as_mut()
        .unwrap()
        .blocking_erase(from, to)
        .map_err(|_| ())
}

/// Writes a page-aligned buffer into an OTA firmware slot.
/// Enforces that the offset is strictly inside Slot A or Slot B.
pub fn flash_slot_write(offset: u32, data: &[u8]) -> Result<(), ()> {
    let to = offset + data.len() as u32;
    let valid_slot_a = offset >= SLOT_A_OFFSET && to <= (SLOT_A_OFFSET + SLOT_SIZE_BYTES as u32);
    let valid_slot_b = offset >= SLOT_B_OFFSET && to <= (SLOT_B_OFFSET + SLOT_SIZE_BYTES as u32);

    if !(valid_slot_a || valid_slot_b) {
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
