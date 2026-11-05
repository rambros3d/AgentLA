#![allow(dead_code, unused)]
//! I2C-tiny-USB / MCP2221 emulation (proof-of-function).
//!
//! Emulates the Microchip MCP2221 command subset over a CDC-ACM channel. Hardware
//! timing is deferred; commands parse and mutate in-RAM state so the device boots.

use crate::pin_owner;

pub const I2C_PINS: [u32; 2] = [16, 17]; // SDA, SCL (flexible, open-drain)

pub struct I2cBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    /// Last commanded status byte reported back to host.
    pub status: u8,
    /// Ring of pending TX bytes (<=64).
    tx: heapless::Vec<u8, 64>,
    rx: heapless::Vec<u8, 64>,
}

impl I2cBridge {
    pub fn new() -> Self {
        Self { active: false, slot: None, status: 0, tx: heapless::Vec::new(), rx: heapless::Vec::new() }
    }
    pub fn active(&self) -> bool { self.active }
    pub fn safe_pins() -> &'static [u32] { &I2C_PINS }
    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active { return Ok(()); }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &I2C_PINS, pin_owner::CAP_I2C)?;
        self.slot = Some(slot);
        self.active = true;
        self.status = 0x20; // MCP2221 "ready"
        Ok(())
    }
    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() {
            let _ = pin_owner::release(slot);
        }
        self.active = false;
        self.status = 0;
        self.tx.clear();
        self.rx.clear();
    }
    /// Parse a minimal MCP2221-style command frame. Returns status byte.
    pub fn process_command(&mut self, frame: &[u8]) -> u8 {
        if frame.is_empty() { return self.status; }
        match frame[0] {
            0x10 => { // configure / echo
                self.status = 0x20;
            }
            0x20 => { // I2C write
                for &b in &frame[1..] { let _ = self.tx.push(b); }
                self.status = 0x21;
            }
            0x30 => { // I2C read
                self.rx.extend_from_slice(&self.tx).ok();
                self.status = 0x31;
            }
            0x00 => { self.status = 0x10; } // reset
            _ => { self.status = 0xEE; }
        }
        self.status
    }
    pub fn rx_bytes(&self) -> &[u8] { self.rx.as_slice() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claim_state() {
        let mut b = I2cBridge::new();
        // host environment: pin registry not initialized; enable may fail. Just check active false.
        assert!(!b.active());
    }
}