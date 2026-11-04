pub mod algo;
pub mod dap;
pub mod pins;
pub mod pio_programs;
pub mod rvswd;
pub mod swd;
pub mod swio;
pub mod target;

use algo::FlashAlgorithmRunner;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use pins::ProgrammerPins;
use target::cortex_m::CortexM;
use target::targets::TargetInfo;
use target::TargetError;

pub static PROGRAMMER_PINS: Mutex<CriticalSectionRawMutex, RefCell<ProgrammerPins>> =
    Mutex::new(RefCell::new(ProgrammerPins::new()));

pub fn detect_target_global() -> Result<TargetInfo, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        target::detect_target(&mut *pins)
    })
}

pub fn halt_target_global() -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd(1000);
        swd_session.jtag_to_swd();
        let mut cortex = CortexM::new(&mut swd_session.transport);
        let res = cortex.halt().map_err(|_| TargetError::Timeout);
        *pins = swd_session.release();
        res
    })
}

pub fn resume_target_global() -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd(1000);
        swd_session.jtag_to_swd();
        let mut cortex = CortexM::new(&mut swd_session.transport);
        let res = cortex.resume().map_err(|_| TargetError::Timeout);
        *pins = swd_session.release();
        res
    })
}

pub fn read_flash_global(addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd(1000);
        swd_session.jtag_to_swd();
        let mut cortex = CortexM::new(&mut swd_session.transport);
        let res = cortex
            .mem_read_block(addr, buf)
            .map_err(|_| TargetError::ProtocolError);
        *pins = swd_session.release();
        res
    })
}

pub fn flash_target_global(addr: u32, data: &[u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        // 1. Detect target
        let info = target::detect_target(&mut *pins)?;

        // 2. Perform flash write
        let mut swd_session = pins.into_swd(1000);
        swd_session.jtag_to_swd();
        let mut cortex = CortexM::new(&mut swd_session.transport);
        let res = FlashAlgorithmRunner::flash_target(&mut cortex, &info, addr, data)
            .map_err(|_| TargetError::FlashError);
        *pins = swd_session.release();
        res
    })
}
