pub mod cortex_m;
pub mod qingke;
pub mod targets;

pub use cortex_m::CortexM;
pub use qingke::QingKe;
pub use targets::{identify_cortex_target, TargetFamily, TargetInfo};

use crate::programmer::pins::ProgrammerPins;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    NoTarget,
    ProtocolError,
    Timeout,
    FlashError,
}

pub fn detect_target(pins: &mut ProgrammerPins) -> Result<TargetInfo, TargetError> {
    // 1. Try ARM SWD
    let mut swd_session = pins.into_swd(1000);
    swd_session.jtag_to_swd();

    let mut dpidr = 0u32;
    if swd_session.read_dpidr_retry(&mut dpidr).is_ok()
        && dpidr != 0
        && dpidr != 0xFFFFFFFF
    {
        let mut cortex = CortexM::new(&mut swd_session.transport);

        // Try reading DBGMCU or sysinfo registers
        let mut dbgmcu_id = 0u32;
        if let Ok(id) = cortex.mem_read32(0xE0042000) {
            dbgmcu_id = id;
        } else if let Ok(id) = cortex.mem_read32(0x40015800) {
            dbgmcu_id = id;
        } else if let Ok(id) = cortex.mem_read32(0xE0044000) {
            dbgmcu_id = id;
        }

        let mut sysinfo_id = 0u32;
        if let Ok(id) = cortex.mem_read32(0x40000000) {
            sysinfo_id = id;
        }

        let target = identify_cortex_target(dpidr, dbgmcu_id, sysinfo_id);
        *pins = swd_session.release();
        return Ok(target);
    }
    *pins = swd_session.release();

    // 2. Try WCH 1-Wire SWIO
    let swio_session = pins.into_swio();
    if let Ok(dmstatus) = swio_session.get(0x11) {
        if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
            let part_id = swio_session.get(0x7F).unwrap_or(dmstatus);
            let info = targets::ch32v003_info(part_id);
            *pins = swio_session.release();
            return Ok(info);
        }
    }
    *pins = swio_session.release();

    // 3. Try WCH 2-Wire RVSWD
    let rvswd_session = pins.into_rvswd();
    if let Ok(dmstatus) = rvswd_session.read(0x11) {
        if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
            let info = targets::ch32v203_info(dmstatus);
            *pins = rvswd_session.release();
            return Ok(info);
        }
    }
    *pins = rvswd_session.release();

    Err(TargetError::NoTarget)
}
