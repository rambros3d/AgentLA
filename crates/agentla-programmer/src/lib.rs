#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod algo;
pub mod dap;
pub mod delay;
pub mod pio;
pub mod rvswd;
pub mod swd;
pub mod swio;
pub mod target;
pub mod traits;

pub use algo::{AlgoBlobHeader, AlgoError, FlashAlgorithmRunner};
pub use dap::CmsisDapHandler;
pub use delay::delay_cycles;
#[cfg(feature = "rp2350-pio")]
pub use pio::{RvswdTransport, SwdTransport, SwioTransport};
pub use rvswd::{RvswdEngine, RvswdError};
pub use swd::{Adiv5Access, Adiv5Engine, SwdError};
pub use swio::{SwioEngine, SwioError};
pub use target::{
    ch32v003_info, ch32v203_info, identify_cortex_target, CortexError, CortexM, QingKe,
    QingKeError, QingKeFamily, QingKeWire, TargetError, TargetFamily, TargetInfo,
};
pub use traits::{RvswdIo, SwdIo, SwioIo};
