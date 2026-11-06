//! Helper for Texas Instruments TXS0108E 8-Bit Bidirectional Voltage-Level Translator.
//!
//! Provides Output Enable (OE) control for active multi-rail level translation (1.2V–5.0V).

/// TXS0108E level translator hardware control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Txs0108Config {
    /// Active-high Output Enable (OE) pin.
    pub oe_pin: u32,
}

impl Txs0108Config {
    pub const fn new(oe_pin: u32) -> Self {
        Self { oe_pin }
    }
}
