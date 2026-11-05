#![allow(dead_code, unused)]
//! SLCAN (Lawicel) ASCII CAN bridge.
//!
//! Maps SLCAN `S`/`O`/`C`/`t`/`T`/`r`/`R` style commands onto a dynamic CAN
//! adapter claim. Hardware transmission is deferred; commands parse and mutate
//! in-RAM state so the device boots and the host tooling can drive the API.

use crate::pin_owner;
use rp_pac as pac;

/// SLCAN claims the CAN-TX (GPIO 18) and CAN-RX (GPIO 19) flexible pair.
pub const SLCAN_PINS: [u32; 2] = [18, 19];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SlcanSpeed { S0 = 0, S1 = 1, S2 = 2, S3 = 3, S4 = 4, S5 = 5, S6 = 6, S7 = 7, S8 = 8 }

pub struct SlcanBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub speed: SlcanSpeed,
    pub open: bool,
    /// Last parsed status text for host reads.
    pub last: heapless::String<64>,
    tx_frames: u32,
    rx_frames: u32,
}

impl SlcanBridge {
    pub fn new() -> Self {
        Self { active: false, slot: None, speed: SlcanSpeed::S5, open: false,
            last: heapless::String::new(), tx_frames: 0, rx_frames: 0 }
    }
    pub fn active(&self) -> bool { self.active }
    pub fn tx_count(&self) -> u32 { self.tx_frames }
    pub fn rx_count(&self) -> u32 { self.rx_frames }

    /// Safety guard: only the documented flexible pair may be claimed.
    pub fn safe_pins() -> &'static [u32] { &SLCAN_PINS }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active { return Ok(()); }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &SLCAN_PINS, pin_owner::CAP_SIO)?;
        self.slot = Some(slot);
        self.active = true;
        self.open = false;
        unsafe {
            // Configure CAN_TX (GPIO 18) as output driven high (recessive idle)
            pac::IO_BANK0.gpio(18).ctrl().write(|w| {
                w.set_funcsel(5);
            });
            pac::PADS_BANK0.gpio(18).modify(|w| {
                w.set_iso(false);
                w.set_ie(false);
                w.set_od(false);
                w.set_pue(false);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << 18);
            pac::SIO.gpio_out(0).value_set().write_value(1 << 18);

            // Configure CAN_RX (GPIO 19) as input with pull-up
            pac::IO_BANK0.gpio(19).ctrl().write(|w| {
                w.set_funcsel(5);
            });
            pac::PADS_BANK0.gpio(19).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_clr().write_value(1 << 19);
        }
        let _ = self.last.push_str("SLCAN_READY");
        Ok(())
    }
    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() { let _ = pin_owner::release(slot); }
        self.active = false;
        self.open = false;
        self.last.clear();
    }

    /// Parse one SLCAN line (without CR). Returns reply text.
    pub fn process_line(&mut self, line: &str) -> heapless::String<64> {
        let mut reply = heapless::String::<64>::new();
        if !self.active {
            let _ = reply.push_str("ERR_NOT_ACTIVE");
            return reply;
        }
        let bytes = line.as_bytes();
        if bytes.is_empty() { return reply; }
        match bytes[0] {
            b'S' => {
                // S0..S8
                let n = bytes.get(1).and_then(|b| (*b as char).to_digit(10)).unwrap_or(5) as u8;
                self.speed = match n { 0=>SlcanSpeed::S0,1=>SlcanSpeed::S1,2=>SlcanSpeed::S2,3=>SlcanSpeed::S3,4=>SlcanSpeed::S4,5=>SlcanSpeed::S5,6=>SlcanSpeed::S6,7=>SlcanSpeed::S7,_=>SlcanSpeed::S8 };
                let _ = reply.push_str("OK");
            }
            b'O' => { self.open = true; let _ = reply.push_str("OK"); }
            b'C' => { self.open = false; let _ = reply.push_str("OK"); }
            b't' | b'T' => {
                if self.open { self.tx_frames = self.tx_frames.saturating_add(1); let _ = reply.push_str("z"); }
                else { let _ = reply.push_str("ERR_CLOSED"); }
            }
            b'r' | b'R' => {
                if self.open { self.rx_frames = self.rx_frames.saturating_add(1); let _ = reply.push_str("z"); }
                else { let _ = reply.push_str("ERR_CLOSED"); }
            }
            _ => { let _ = reply.push_str("ERR_CMD"); }
        }
        self.last = reply.clone();
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_inactive() {
        let b = SlcanBridge::new();
        assert!(!b.active());
    }
}
