//! Nordic nRF recovery via CTRL-AP mass erase.

pub use agentla_programmer::target::nrf::*;
use crate::programmer::swd::SwdTransport;
use crate::programmer::target::cortex_m::CortexM;

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
