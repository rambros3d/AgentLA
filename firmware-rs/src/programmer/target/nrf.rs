use crate::programmer::swd::SwdTransport;
use crate::programmer::target::cortex_m::CortexM;

pub const NRF_CTRL_AP: u8 = 1;
pub const CTRL_AP_RESET: u8 = 0x00;
pub const CTRL_AP_ERASEALL: u8 = 0x04;
pub const CTRL_AP_ERASEALLSTATUS: u8 = 0x08;
pub const CTRL_AP_APPROTECTSTATUS: u8 = 0x0C;
pub const CTRL_AP_IDR: u8 = 0xFC;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NrfRescueError {
    SwdError,
    CtrlApNotFound,
    EraseTimeout,
    TargetNotResponding,
}

/// Unprotect and mass erase locked Nordic nRF52/nRF53 devices via CoreSight CTRL-AP (AP #1)
pub fn nrf_ctrl_ap_erase_all(cortex: &mut CortexM) -> Result<(), NrfRescueError> {
    // 1. Verify CTRL-AP exists by reading IDR on AP #1
    let idr = cortex
        .read_ap_reg(NRF_CTRL_AP, CTRL_AP_IDR)
        .map_err(|_| NrfRescueError::SwdError)?;

    // Nordic CTRL-AP IDR is typically 0x02880000, 0x12880000, or 0x22880000
    if (idr & 0x00FFFFFF) != 0x00880000 && idr == 0 {
        return Err(NrfRescueError::CtrlApNotFound);
    }

    // 2. Trigger mass erase by writing 0x01 to ERASEALL register (0x04)
    cortex
        .write_ap_reg(NRF_CTRL_AP, CTRL_AP_ERASEALL, 0x01)
        .map_err(|_| NrfRescueError::SwdError)?;

    // 3. Poll ERASEALLSTATUS (0x08) until it becomes 0 (idle/complete)
    let mut erase_done = false;
    for _ in 0..300 {
        if let Ok(status) = cortex.read_ap_reg(NRF_CTRL_AP, CTRL_AP_ERASEALLSTATUS) {
            if (status & 0x01) == 0 {
                erase_done = true;
                break;
            }
        }
        cortex_m::asm::delay(1_500_000); // 10 ms delay
    }

    if !erase_done {
        return Err(NrfRescueError::EraseTimeout);
    }

    // 4. Reset target via CTRL-AP RESET register
    let _ = cortex.write_ap_reg(NRF_CTRL_AP, CTRL_AP_RESET, 0x01);
    cortex_m::asm::delay(150_000); // 1 ms
    let _ = cortex.write_ap_reg(NRF_CTRL_AP, CTRL_AP_RESET, 0x00);
    cortex_m::asm::delay(150_000); // 1 ms

    // 5. Restore AHB-AP selection (AP #0)
    let _ = cortex.select_ap(0);
    Ok(())
}

/// Global entry point for Nordic rescue operation
pub fn rescue_nrf_global() -> Result<(), NrfRescueError> {
    let mut swd = SwdTransport::new();
    swd.init(1000);
    swd.jtag_to_swd();
    let mut dpidr = 0u32;
    let _ = swd.read_dpidr_retry(&mut dpidr);
    let _ = swd.power_up_debug();

    let mut cortex = CortexM::new(&mut swd);
    nrf_ctrl_ap_erase_all(&mut cortex)
}
