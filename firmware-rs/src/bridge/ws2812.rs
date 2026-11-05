#![allow(dead_code, unused)]
//! WS2812 / NeoPixel PIO bridge.
//!
//! Drives a single-wire addressable LED strip via a PIO state machine. Timing is
//! deferred; the API models pixel buffers and transfers in RAM.

use crate::pin_owner;

/// Single data line for the LED strip (flexible, PIO-capable).
pub const WS2812_PINS: [u32; 1] = [11];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pixel { pub r: u8, pub g: u8, pub b: u8 }

pub const MAX_PIXELS: usize = 8;

pub struct Ws2812Bridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pin: u32,
    pixels: heapless::Vec<Pixel, MAX_PIXELS>,
    transmitted: u32,
}

impl Ws2812Bridge {
    pub fn new() -> Self {
        Self { active: false, slot: None, pin: WS2812_PINS[0], pixels: heapless::Vec::new(), transmitted: 0 }
    }
    pub fn active(&self) -> bool { self.active }
    pub fn pixel_count(&self) -> usize { self.pixels.len() }
    pub fn transmitted(&self) -> u32 { self.transmitted }

    pub fn safe_pins() -> &'static [u32] { &WS2812_PINS }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active { return Ok(()); }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &WS2812_PINS, pin_owner::CAP_PIO)?;
        self.slot = Some(slot);
        self.active = true;
        self.pixels.clear();
        Ok(())
    }
    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() { let _ = pin_owner::release(slot); }
        self.active = false;
        self.pixels.clear();
    }

    pub fn set_pixel(&mut self, index: usize, p: Pixel) -> Result<(), ()> {
        if !self.active { return Err(()); }
        if index >= MAX_PIXELS { return Err(()); }
        if self.pixels.len() <= index {
            // pad with black
            while self.pixels.len() <= index {
                let _ = self.pixels.push(Pixel { r: 0, g: 0, b: 0 });
            }
        }
        self.pixels[index] = p;
        Ok(())
    }
    pub fn clear(&mut self) {
        self.pixels.clear();
    }
    /// Record a transmit of the current buffer (hardware DMA deferred).
    pub fn transmit(&mut self) -> Result<u32, ()> {
        if !self.active { return Err(()); }
        self.transmitted = self.transmitted.saturating_add(self.pixels.len() as u32);
        Ok(self.pixels.len() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_inactive() {
        let b = Ws2812Bridge::new();
        assert!(!b.active());
    }
}
