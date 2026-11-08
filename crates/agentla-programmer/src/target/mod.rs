//! Target processor architecture drivers and identification.

pub mod cortex_m;
pub mod nrf;
pub mod qingke;
pub mod targets;

pub use cortex_m::{CortexError, CortexM};
pub use nrf::{nrf_ctrl_ap_erase_all, NrfRescueError};
pub use qingke::{QingKe, QingKeError, QingKeFamily, QingKeWire};
pub use targets::{
    ch32v003_info, ch32v203_info, identify_cortex_target, TargetFamily, TargetInfo,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    NoTarget,
    ProtocolError,
    Timeout,
    FlashError,
    FlashErrorCode(u32),
}
