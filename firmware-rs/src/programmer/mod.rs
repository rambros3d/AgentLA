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
pub use pins::TargetPort;
use pins::ProgrammerPins;
use target::cortex_m::CortexM;
use target::qingke::QingKe;
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

pub fn detect_target_port_global(port: TargetPort) -> Result<TargetInfo, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        target::detect_target_port(&mut *pins, port)
    })
}

pub fn debug_probe_bluepill_global() -> (u32, u8, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
        swd_session.jtag_to_swd();
        let mut dpidr = 0u32;
        let res = swd_session.read_dpidr_retry(&mut dpidr);
        let req = swd::SwdTransport::build_request(false, true, 0);
        let ack = if res.is_ok() { swd::SWD_ACK_OK } else { 7 };
        *pins = swd_session.release();
        (dpidr, ack, req as u32)
    })
}

pub fn debug_probe_ch32v203_global() -> (u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
        let dmstatus = rvswd_session.read(0x11).unwrap_or(0xDEADBEEF);
        *pins = rvswd_session.release();
        (dmstatus, 0)
    })
}

pub fn debug_probe_ch32v003_global() -> (u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let swio_session = pins.into_swio_port(TargetPort::Ch32v003);
        let dmstatus = swio_session.get(0x11).unwrap_or(0xDEADBEEF);
        let partid = swio_session.get(0x7F).unwrap_or(0xDEADBEEF);
        *pins = swio_session.release();
        (dmstatus, partid)
    })
}

pub fn halt_target_port_global(port: TargetPort) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex.halt().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.halt().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk.halt().map_err(|_| TargetError::Timeout);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn resume_target_port_global(port: TargetPort) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex.resume().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn read_flash_port_global(port: TargetPort, addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex
                    .mem_read_block(addr, buf)
                    .map_err(|_| TargetError::ProtocolError);
                *pins = swd_session.release();
                res
            }
            _ => Err(TargetError::ProtocolError),
        }
    })
}

pub fn halt_target_global() -> Result<(), TargetError> {
    halt_target_port_global(TargetPort::BluePill)
}

pub fn resume_target_global() -> Result<(), TargetError> {
    resume_target_port_global(TargetPort::BluePill)
}

pub fn read_flash_global(addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    read_flash_port_global(TargetPort::BluePill, addr, buf)
}

pub fn flash_target_global(addr: u32, data: &[u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        // 1. Detect target
        let info = target::detect_target_port(&mut *pins, TargetPort::BluePill)?;

        // 2. Perform flash write
        let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
        swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
        let mut cortex = CortexM::new(&mut swd_session.transport);
        let res = FlashAlgorithmRunner::flash_target(&mut cortex, &info, addr, data)
            .map_err(|_| TargetError::FlashError);
        *pins = swd_session.release();
        res
    })
}
