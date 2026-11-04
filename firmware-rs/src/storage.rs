use embassy_rp::flash::{Blocking, Flash, FLASH_BASE};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use littlefs2::driver::Storage;
use littlefs2::fs::{Allocation, Filesystem};
use static_cell::StaticCell;

pub const LITTLEFS_START_OFFSET: u32 = 0x00190000; // 1.5625 MB offset
pub const PARTITION_SIZE_BYTES: usize = 1024 * 1024; // 1 MB partition
pub const BLOCK_SIZE: usize = 4096;
pub const BLOCK_COUNT: usize = PARTITION_SIZE_BYTES / BLOCK_SIZE; // 256 blocks

pub struct FlashStorage {
    flash: Flash<'static, FLASH, Blocking, 4_194_304>,
    offset: u32,
}

impl FlashStorage {
    pub fn new(flash: Flash<'static, FLASH, Blocking, 4_194_304>, offset: u32) -> Self {
        Self { flash, offset }
    }
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
        // RP2350 supports fast zero-copy XIP read from memory space
        let flash_ptr = (FLASH_BASE as usize + abs_off as usize) as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(flash_ptr, buf.as_mut_ptr(), buf.len());
        }
        Ok(buf.len())
    }

    fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
        let abs_off = self.offset + off as u32;
        self.flash
            .blocking_write(abs_off, data)
            .map_err(|_| littlefs2::io::Error::IO)?;
        Ok(data.len())
    }

    fn erase(&mut self, off: usize, len: usize) -> littlefs2::io::Result<usize> {
        let from = self.offset + off as u32;
        let to = from + len as u32;
        self.flash
            .blocking_erase(from, to)
            .map_err(|_| littlefs2::io::Error::IO)?;
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
}

pub static FS_MUTEX: Mutex<CriticalSectionRawMutex, Option<SafeFilesystem>> = Mutex::new(None);

pub fn init_filesystem(flash: Flash<'static, FLASH, Blocking, 4_194_304>) -> bool {
    static ALLOC: StaticCell<Allocation<FlashStorage>> = StaticCell::new();
    static STORAGE: StaticCell<FlashStorage> = StaticCell::new();

    let storage = STORAGE.init(FlashStorage::new(flash, LITTLEFS_START_OFFSET));
    let alloc = ALLOC.init(Allocation::new());

    let mounted = Filesystem::mount_or_else(alloc, storage, |_err, st, _al| {
        defmt::warn!("Formatting uninitialized LittleFS partition...");
        Filesystem::format(st)
    });

    match mounted {
        Ok(fs) => {
            defmt::info!(
                "LittleFS partition mounted at offset 0x{:08x}",
                LITTLEFS_START_OFFSET
            );
            let mut guard = FS_MUTEX.try_lock().unwrap();
            *guard = Some(SafeFilesystem(fs));
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
