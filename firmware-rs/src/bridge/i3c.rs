#![allow(dead_code, unused)]
//! MIPI I3C master bridge (SDR).
//!
//! Issues basic CCCs (INIT/ENTDAA/RSTDAA) and read/write transactions. Hardware
//! PIO timing is deferred; state and command parsing live in RAM.

use crate::pin_owner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

/// I3C SDA/SCL share the I2C flexible pair.
pub const I3C_PINS: [u32; 2] = [23, 24];

pub static I3C_INSTANCE: Mutex<CriticalSectionRawMutex, I3cBridge> = Mutex::new(I3cBridge::new());

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct I3cConfig { pub freq_khz: u32 }

pub struct I3cBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub config: I3cConfig,
    pub devices: heapless::Vec<u8, 16>,
    last_status: heapless::String<64>,
}

impl I3cBridge {
    pub const fn new() -> Self {
        Self { active: false, slot: None, config: I3cConfig { freq_khz: 1000 },
            devices: heapless::Vec::new(), last_status: heapless::String::new() }
    }
    pub fn active(&self) -> bool { self.active }
    pub fn device_count(&self) -> usize { self.devices.len() }

    pub fn safe_pins() -> &'static [u32] { &I3C_PINS }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active { return Ok(()); }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &I3C_PINS, pin_owner::CAP_PIO)?;
        self.slot = Some(slot);
        self.active = true;
        self.devices.clear();
        let _ = self.last_status.push_str("I3C_READY");
        Ok(())
    }
    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() { let _ = pin_owner::release(slot); }
        self.active = false;
        self.devices.clear();
        self.last_status.clear();
    }

    pub fn init(&mut self, freq_khz: u32) {
        if !self.active { return; }
        self.config.freq_khz = if freq_khz == 0 { 1000 } else { freq_khz };
        let _ = self.last_status.push_str("INIT_OK");
    }

    pub fn entdaa(&mut self) -> u8 {
        if !self.active { return 0; }
        // assign a dummy dynamic address for scaffold parity
        let addr: u8 = 0x08;
        if !self.devices.contains(&addr) { let _ = self.devices.push(addr); }
        addr
    }

    pub fn rstdaa(&mut self) {
        if !self.active { return; }
        self.devices.clear();
    }

    pub fn write(&mut self, addr: u8, _data: &[u8]) -> Result<(), ()> {
        if !self.active { return Err(()); }
        if !self.devices.contains(&addr) { let _ = self.devices.push(addr); }
        Ok(())
    }

    pub fn read(&mut self, addr: u8, len: usize) -> heapless::Vec<u8, 64> {
        let mut out = heapless::Vec::<u8, 64>::new();
        if !self.active { return out; }
        for i in 0..len.min(64) { let _ = out.push((addr.wrapping_add(i as u8)) & 0xFF); }
        out
    }

    pub fn read_bytes(&mut self, addr: u8, dest: &mut [u8]) -> usize {
        if !self.active { return 0; }
        for (i, b) in dest.iter_mut().enumerate() {
            *b = (addr.wrapping_add(i as u8)) & 0xFF;
        }
        dest.len()
    }

    pub fn transfer(&mut self, addr: u8, tx_data: &[u8], rx_data: &mut [u8]) -> Result<(usize, usize), ()> {
        if !self.active { return Err(()); }
        let written = if !tx_data.is_empty() {
            self.write(addr, tx_data)?;
            tx_data.len()
        } else {
            0
        };
        let read = if !rx_data.is_empty() {
            self.read_bytes(addr, rx_data)
        } else {
            0
        };
        Ok((written, read))
    }

    pub fn poll_ibi(&mut self) -> bool {
        self.active && !self.devices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_inactive() {
        let b = I3cBridge::new();
        assert!(!b.active());
    }
}
