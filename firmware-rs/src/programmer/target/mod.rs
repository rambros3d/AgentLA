pub mod cortex_m;
pub mod nrf;
pub mod qingke;
pub mod targets;

pub use cortex_m::CortexM;
pub use nrf::rescue_nrf_global;
pub use qingke::QingKe;
pub use targets::{identify_cortex_target, TargetFamily, TargetInfo};

use crate::programmer::pins::{ProgrammerPins, TargetPort};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    NoTarget,
    ProtocolError,
    Timeout,
    FlashError,
    FlashErrorCode(u32),
}

pub fn detect_target_port(pins: &mut ProgrammerPins, port: TargetPort) -> Result<TargetInfo, TargetError> {
    match port {
        TargetPort::BluePill => {
            // ARM SWD on GPIO 4 (SWCLK), 5 (SWDIO)
            let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
            if let Ok(dpidr) = swd_session.attach() {
                let mut cortex = CortexM::new(&mut swd_session.transport);

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
            Err(TargetError::NoTarget)
        }
        TargetPort::Ch32v203 => {
            // First try WCH 2-Wire RVSWD on GPIO 8 (SWCLK), 9 (SWDIO)
            let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
            if let Ok(dmstatus) = rvswd_session.attach() {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    let info = targets::ch32v203_info(dmstatus);
                    *pins = rvswd_session.release();
                    return Ok(info);
                }
            }
            *pins = rvswd_session.release();

            // Next try SWD on GPIO 8/9
            let mut swd_session = pins.into_swd_port(TargetPort::Ch32v203, 1000);
            swd_session.jtag_to_swd();
            let mut dpidr = 0u32;
            if swd_session.read_dpidr_retry(&mut dpidr).is_ok()
                && dpidr != 0
                && dpidr != 0xFFFFFFFF
            {
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let dbgmcu_id = cortex.mem_read32(0xE0042000).unwrap_or(0);
                let sysinfo_id = cortex.mem_read32(0x40000000).unwrap_or(0);
                let target = identify_cortex_target(dpidr, dbgmcu_id, sysinfo_id);
                *pins = swd_session.release();
                return Ok(target);
            }
            *pins = swd_session.release();

            // Next try 1-wire SWIO on GPIO 9
            let mut swio_session = pins.into_swio_port(TargetPort::Ch32v203);
            if let Ok(dmstatus) = swio_session.attach() {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    let part_id = swio_session.get(0x7F).unwrap_or(dmstatus);
                    let info = targets::ch32v203_info(part_id);
                    *pins = swio_session.release();
                    return Ok(info);
                }
            }
            *pins = swio_session.release();

            Err(TargetError::NoTarget)
        }
        TargetPort::Ch32v003 => {
            // WCH 1-Wire SWIO on GPIO 12
            let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
            if let Ok(dmstatus) = swio_session.attach() {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    let part_id = swio_session.get(0x7F).unwrap_or(dmstatus);
                    let info = targets::ch32v003_info(part_id);
                    *pins = swio_session.release();
                    return Ok(info);
                }
            }
            *pins = swio_session.release();

            Err(TargetError::NoTarget)
        }
    }
}

pub fn detect_target(pins: &mut ProgrammerPins) -> Result<TargetInfo, TargetError> {
    if let Ok(target) = detect_target_port(pins, TargetPort::BluePill) {
        return Ok(target);
    }
    if let Ok(target) = detect_target_port(pins, TargetPort::Ch32v203) {
        return Ok(target);
    }
    if let Ok(target) = detect_target_port(pins, TargetPort::Ch32v003) {
        return Ok(target);
    }
    Err(TargetError::NoTarget)
}
