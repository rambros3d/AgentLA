#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod gdb;
pub mod rtt;
pub mod target;

pub use target::{BreakpointError, GdbTarget, MemoryAccess};
