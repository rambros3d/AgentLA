//! # AgentLA Board Support Package (BSP) Core Library
//!
//! Provides the core [`BoardProfile`] trait, hardware zone types, and reusable
//! external IC drivers (TI PGA113, TI INA180, TCAN334, TXS0108E) for the AgentLA
//! hardware ecosystem.

#![no_std]

pub mod drivers;
pub mod traits;

pub use drivers::{
    ina180::{Ina180, Ina180Gain},
    pga113::{Pga113Channel, Pga113Config, Pga113Gain},
    tcan334::Tcan334Config,
    txs0108::Txs0108Config,
};

pub use traits::{
    AdapterPins, AnalogFrontendConfig, BoardFeatures, BoardProfile, CanPins, FlexiblePool,
    I2cPins, McuPackage, ProgrammerPins, Rs485Pins, SpiPins, UartPins,
};
