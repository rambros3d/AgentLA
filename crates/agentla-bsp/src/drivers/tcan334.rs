//! Driver / controller configuration for Texas Instruments TCAN334 CAN-FD Transceiver.
//!
//! Supports differential CAN-FD transmission and optional firmware-switchable 120 Ohm termination.

/// TCAN334 transceiver configuration and pin control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tcan334Config {
    pub tx_pin: u32,
    pub rx_pin: u32,
    pub term_relay_pin: Option<u32>,
}

impl Tcan334Config {
    pub const fn new(tx_pin: u32, rx_pin: u32, term_relay_pin: Option<u32>) -> Self {
        Self {
            tx_pin,
            rx_pin,
            term_relay_pin,
        }
    }

    /// Check if switchable termination is supported by this hardware wiring.
    pub const fn has_switchable_termination(&self) -> bool {
        self.term_relay_pin.is_some()
    }
}
