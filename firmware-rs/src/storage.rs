use embassy_rp::flash::{Blocking, Flash, FLASH_BASE};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use littlefs2::driver::Storage;
use littlefs2::fs::{Allocation, Filesystem};
use static_cell::StaticCell;

pub const LITTLEFS_START_OFFSET: u32 = 0x00190000; // 1.5625 MB offset (profile slots live below this)
pub const PARTITION_SIZE_BYTES: usize = 2 * 1024 * 1024; // 2 MB partition
pub const BLOCK_SIZE: usize = 4096;
pub const BLOCK_COUNT: usize = PARTITION_SIZE_BYTES / BLOCK_SIZE; // 512 blocks

/// Profile slots window: 32 × 2 KiB slots (this change), directly below the LittleFS
/// partition. Only `crate::profile` may touch bytes inside it.
pub const PROFILE_WINDOW_START: u32 = 0x00180000;
/// Exclusive upper bound of the profile window = first byte of the LittleFS partition.
pub const PROFILE_WINDOW_END: u32 = LITTLEFS_START_OFFSET;

/// The `FLASH` peripheral is a single instance: the physical flash object lives in this
/// mutex from [`init_filesystem`] on, shared by the LittleFS storage below and the
/// narrow profile-window primitives ([`profile_flash_read`], [`profile_flash_write`],
/// [`profile_flash_erase`], task 2.1). Never nested: profile ops take this mutex inside
/// their own `PROFILE_MUTEX` (profile.rs); LittleFS ops take it per-storage-call while
/// `FS_MUTEX` is held.
static FLASH_MUTEX: Mutex<CriticalSectionRawMutex, Option<FlashHandle>> = Mutex::new(None);

pub type FlashHandle = Flash<'static, FLASH, Blocking, 4_194_304>;

/// LittleFS storage for the 2 MB partition. The physical `Flash` peripheral does **not**
/// live here anymore — access is taken per-operation from [`FLASH_MUTEX`] (single
/// instance, shared with the profile-window primitives).
pub struct FlashStorage {
    offset: u32,
}

fn flash_guard() -> embassy_sync::mutex::MutexGuard<'static, CriticalSectionRawMutex, Option<FlashHandle>> {
    // Under the single-threaded executor, sync sections never yield while holding this
    // guard, so try_lock cannot fail (except on a forgotten init_filesystem — a boot bug).
    let mut guard = FLASH_MUTEX
        .try_lock()
        .expect("FLASH mutex unavailable (init_filesystem not called or nested op)");
    guard
        .as_mut()
        .expect("FLASH peripheral not initialized at init_filesystem");
    guard
}

impl Storage for FlashStorage {
    const READ_SIZE: usize = 16;
    const WRITE_SIZE: usize = 256;
    const BLOCK_SIZE: usize = 4096;
    const BLOCK_COUNT: usize = BLOCK_COUNT;
    const BLOCK_CYCLES: isize = 500;

    type CACHE_SIZE = littlefs2::consts::U256;
    type LOOKAHEAD_SIZE = littlefs2::consts::U64;

    fn read(&mut self, off: usize, buf: &mut [u8]) -> littlefs2::io::Result<usize> {
        let abs_off = self.offset + off as u32;
        debug_assert!(abs_off >= PROFILE_WINDOW_END, "littlefs read below partition");
        // RP2350 supports fast zero-copy XIP read from memory space
        let flash_ptr = (FLASH_BASE as usize + abs_off as usize) as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
        let abs_off = self.offset + off as u32;
        debug_assert!(abs_off >= PROFILE_WINDOW_END, "littlefs write below partition");
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
        debug_assert!(from >= PROFILE_WINDOW_END, "littlefs erase below partition");
        flash_guard().as_mut().unwrap().blocking_erase(from, to).unwrap();
        Ok(len)
    }
}

#[allow(dead_code)]
pub struct SafeFilesystem(pub Filesystem<'static, FlashStorage>);
unsafe impl Send for SafeFilesystem {}
unsafe impl Sync for SafeFilesystem {}

impl SafeFilesystem {
    #[allow(dead_code)]
    pub fn filesystem(&self) -> &Filesystem<'static, FlashStorage> {
        &self.0
    }

    /// Mutable access to the filesystem. All LittleFS convenience methods take
    /// `&self` only, but this is kept for future use.
    #[allow(dead_code)]
    pub unsafe fn filesystem_mut(&mut self) -> &mut Filesystem<'static, FlashStorage> {
        &mut self.0
    }
}

pub static FS_MUTEX: Mutex<CriticalSectionRawMutex, Option<SafeFilesystem>> = Mutex::new(None);

/// Set while `Filesystem::mount_or_else` took the format path (first boot or corrupted fs).
pub static FS_WAS_FORMATTED: portable_atomic::AtomicBool = portable_atomic::AtomicBool::new(false);

/// Run `f` with the mounted filesystem, if any.
pub fn with_fs<R>(f: impl FnOnce(&Filesystem<'static, FlashStorage>) -> R) -> Option<R> {
    let guard = FS_MUTEX.try_lock().ok()?;
    let fs = guard.as_ref()?.filesystem();
    Some(f(fs))
}

pub fn init_filesystem(flash: FlashHandle) -> bool {
    // Take permanent ownership of the FLASH peripheral (uncontended at boot).
    {
        let mut guard = FLASH_MUTEX.try_lock().expect("flash mutex free at boot");
        *guard = Some(flash);
    }

    static ALLOC: StaticCell<Allocation<FlashStorage>> = StaticCell::new();
    static STORAGE: StaticCell<FlashStorage> = StaticCell::new();

    let storage = STORAGE.init(FlashStorage {
        offset: LITTLEFS_START_OFFSET,
    });
    let alloc = ALLOC.init(Allocation::new());

    defmt::info!(
        "Mounting LittleFS partition ({} KiB at offset 0x{:08x})...",
        PARTITION_SIZE_BYTES / 1024,
        LITTLEFS_START_OFFSET
    );

    let mounted = Filesystem::mount_or_else(alloc, storage, |_err, st, _al| {
        defmt::warn!("Formatting uninitialized LittleFS partition (may take a second)...");
        FS_WAS_FORMATTED.store(true, portable_atomic::Ordering::Relaxed);
        Filesystem::format(st)
    });

    match mounted {
        Ok(fs) => {
            defmt::info!(
                "LittleFS partition mounted at offset 0x{:08x} (formatted_on_boot={})",
                LITTLEFS_START_OFFSET,
                FS_WAS_FORMATTED.load(portable_atomic::Ordering::Relaxed)
            );
            let mut guard = FS_MUTEX.try_lock().unwrap();
            *guard = Some(SafeFilesystem(fs));
            // Drop the guard before calling into apps code: `seed_default_apps`
            // uses `with_fs`, which must be able to acquire `FS_MUTEX`.
            drop(guard);
            crate::apps::seed_default_apps();
            true
        }
        Err(e) => {
            defmt::error!(
                "Failed to mount LittleFS partition: {:?}",
                defmt::Debug2Format(&e)
            );
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Profile-window primitives (task 2.1). The ONLY callers are in `crate::profile`;
// every function asserts accesses stay inside `PROFILE_WINDOW_START..PROFILE_WINDOW_END`
// so the region-disjointness with the LittleFS partition is enforced at debug build level.
// ---------------------------------------------------------------------------

/// Zero-copy-speed XIP read of `buf.len()` bytes from the profile window into `buf`.
pub fn profile_flash_read(offset: u32, buf: &mut [u8]) {
    debug_assert!(
        offset >= PROFILE_WINDOW_START
            && (offset as usize + buf.len()) <= PROFILE_WINDOW_END as usize,
        "profile read outside window"
    );
    let ptr = (FLASH_BASE as usize + offset as usize) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), buf.len());
    }
}

/// Erase a `[from, to)` range inside the profile window (4 KiB sectors; save/delete RM
/// uses one sector at a time).
pub fn profile_flash_erase(from: u32, to: u32) {
    debug_assert!(
        from >= PROFILE_WINDOW_START
            && to <= PROFILE_WINDOW_END
            && from < to
            && (from - PROFILE_WINDOW_START) % BLOCK_SIZE as u32 == 0,
        "profile erase outside window / unaligned"
    );
    flash_guard()
        .as_mut()
        .unwrap()
        .blocking_erase(from, to)
        .expect("flash erase");
}

/// Program a range inside the profile window in page-granular 256 B bursts.
pub fn profile_flash_write(offset: u32, data: &[u8]) {
    debug_assert!(
        offset >= PROFILE_WINDOW_START
            && (offset as usize + data.len()) <= PROFILE_WINDOW_END as usize,
        "profile write outside window"
    );
    let mut guard = flash_guard();
    let flash = guard.as_mut().unwrap();
    for (page, chunk) in data.chunks(256).enumerate() {
        flash
            .blocking_write(offset + (page * 256) as u32, chunk)
            .expect("flash program");
    }
}
