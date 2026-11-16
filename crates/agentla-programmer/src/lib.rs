//! Multi-protocol hardware programmer and debugger crate for AgentLA.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod algo;
pub mod clock;
pub mod dap;
pub mod delay;
pub mod pio;
pub mod rvdm;
pub mod rvswd;
pub mod stlink;
pub mod swd;
pub mod swio;
pub mod swo;
pub mod target;
pub mod traits;
pub mod wch;

pub use algo::{AlgoBlobHeader, AlgoError, FlashAlgorithmRunner};
pub use clock::{calculate_pio_clkdiv, swd_divisor_to_khz, swd_khz_to_divisor, wch_speed_to_khz};
pub use dap::CmsisDapHandler;
pub use delay::delay_cycles;
#[cfg(feature = "rp2350-pio")]
pub use pio::{RvswdTransport, SwdTransport, SwioTransport};
pub use rvdm::{Rvdm, RvdmError};
pub use rvswd::{RvswdEngine, RvswdError};
pub use stlink::{format_version, parse_version, StLinkHandler, STLINK_VER_REPORTED};
pub use swd::{Adiv5Access, Adiv5Engine, SwdError};
pub use swio::{SwioEngine, SwioError};
pub use swo::{SwoEngine, SwoTelemetry, SWO_BUF_SIZE, SWO_DEFAULT_BAUD, SWO_MAX_BAUD};
pub use target::{
    ch32v003_info, ch32v203_info, identify_cortex_target, CortexError, CortexM, QingKe,
    QingKeError, QingKeFamily, QingKeWire, TargetError, TargetFamily, TargetInfo,
};
pub use traits::{ProbeWireTopology, RvswdIo, SwdIo, SwioIo};
pub use wch::WchLinkHandler;
pub use agentla_targets as targets;
