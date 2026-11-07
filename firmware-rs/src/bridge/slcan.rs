#![allow(dead_code, unused)]
//! SLCAN (Lawicel) ASCII CAN bridge.
//!
//! Maps SLCAN `S`/`O`/`C`/`t`/`T`/`r`/`R` style commands onto a dynamic CAN
//! adapter claim. Hardware transmission is deferred; commands parse and mutate
//! in-RAM state so the device boots and the host tooling can drive the API.

pub use agentla_bridges::slcan::{
    format_frame, format_frame_string, parse_command, SlcanCommand, SlcanEngine, SlcanError,
    SlcanSpeed,
};

use crate::board::ActiveBoard;
use crate::pin_owner;
use agentla_bsp::BoardProfile;
use rp_pac as pac;

/// SLCAN claims the CAN-TX and CAN-RX pin pair from the active board profile.
pub const SLCAN_PINS: [u32; 2] = [
    match ActiveBoard::ADAPTERS.can {
        Some(can) => can.tx,
        None => 18,
    },
    match ActiveBoard::ADAPTERS.can {
        Some(can) => can.rx,
        None => 19,
    },
];

pub struct SlcanBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub engine: SlcanEngine,
    /// Last parsed status text for host reads.
    pub last: heapless::String<64>,
}

impl Default for SlcanBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl SlcanBridge {
    pub const fn new() -> Self {
        Self {
            active: false,
            slot: None,
            engine: SlcanEngine::new(),
            last: heapless::String::new(),
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn is_open(&self) -> bool {
        self.engine.is_open()
    }

    pub fn speed(&self) -> SlcanSpeed {
        self.engine.speed
    }

    pub fn tx_count(&self) -> u32 {
        self.engine.tx_count()
    }

    pub fn rx_count(&self) -> u32 {
        self.engine.rx_count()
    }

    /// Safety guard: only the documented flexible pair may be claimed.
    pub fn safe_pins() -> &'static [u32] {
        &SLCAN_PINS
    }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active {
            return Ok(());
        }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &SLCAN_PINS, pin_owner::CAP_SIO)?;
        self.slot = Some(slot);
        self.active = true;
        self.engine.open = false;
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
        if let Some(slot) = self.slot.take() {
            let _ = pin_owner::release(slot);
        }
        self.active = false;
        self.engine.open = false;
        self.last.clear();
    }

    /// Parse one SLCAN line (without CR). Returns reply text.
    pub fn process_line(&mut self, line: &str) -> heapless::String<64> {
        let mut reply = heapless::String::<64>::new();
        if !self.active {
            let _ = reply.push_str("ERR_NOT_ACTIVE");
            return reply;
        }
        let res = self.engine.process_line(line);
        self.last = res.clone();
        res
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
