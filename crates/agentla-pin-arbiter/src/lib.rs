#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod arbiter;
pub mod quiesce;
pub mod types;

pub use arbiter::{
    capabilities, mask_from_pins, Claim, PinArbiter, ANALOG_PINS, ANALOG_PIN_COUNT,
    DEFAULT_FIXED_MASK, DEFAULT_FLEX_MASK, FIXED_PINS, FLEX_PINS, FLEX_PIN_COUNT, MAX_CLAIMS,
    MAX_PINS,
};
pub use quiesce::PinQuiesce;
pub use types::{
    ClaimError, ClaimErrorKind, Driver, SlotId, CAP_ADC, CAP_I2C, CAP_PIO, CAP_PWM, CAP_SIO,
    CAP_SPI, CAP_UART,
};
