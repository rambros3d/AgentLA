//! Nordic nRF52/nRF53 device recovery via CTRL-AP mass erase.

use crate::delay::delay_cycles;
use crate::swd::Adiv5Access;
use crate::target::cortex_m::CortexM;

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
pub fn nrf_ctrl_ap_erase_all<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), NrfRescueError> {
    let idr = cortex
        .read_ap_reg(NRF_CTRL_AP, CTRL_AP_IDR)
        .map_err(|_| NrfRescueError::SwdError)?;

    if (idr & 0x00FFFFFF) != 0x00880000 && idr == 0 {
        return Err(NrfRescueError::CtrlApNotFound);
    }

    cortex
        .write_ap_reg(NRF_CTRL_AP, CTRL_AP_ERASEALL, 0x01)
        .map_err(|_| NrfRescueError::SwdError)?;

    let mut erase_done = false;
    for _ in 0..300 {
        if let Ok(status) = cortex.read_ap_reg(NRF_CTRL_AP, CTRL_AP_ERASEALLSTATUS) {
            if (status & 0x01) == 0 {
                erase_done = true;
                break;
            }
        }
        delay_cycles(1_500_000); // 10 ms delay
    }

    if !erase_done {
        return Err(NrfRescueError::EraseTimeout);
    }

    let _ = cortex.write_ap_reg(NRF_CTRL_AP, CTRL_AP_RESET, 0x01);
    delay_cycles(150_000);
    let _ = cortex.write_ap_reg(NRF_CTRL_AP, CTRL_AP_RESET, 0x00);
    delay_cycles(150_000);

    let _ = cortex.select_ap(0);
    Ok(())
}
