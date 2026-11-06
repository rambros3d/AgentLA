#![allow(dead_code, unused)]
//! WS2812 / NeoPixel bridge.
//!
//! Drives addressable single-wire LED strips with cycle-accurate timing
//! directly out of flexible GPIO pins (e.g. GPIO 22).

use crate::board::ActiveBoard;
use crate::pin_owner;
use agentla_bsp::BoardProfile;
use rp_pac as pac;

/// Single data line for the LED strip from active board profile.
pub const WS2812_PINS: [u32; 1] = match ActiveBoard::ADAPTERS.default_ws2812 {
    Some(pin) => [pin],
    None => [22],
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pixel { pub r: u8, pub g: u8, pub b: u8 }

pub const MAX_PIXELS: usize = 64;

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
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &WS2812_PINS, pin_owner::CAP_SIO)?;
        self.slot = Some(slot);
        self.active = true;
        self.pixels.clear();
        let pin = self.pin;
        unsafe {
            pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                w.set_funcsel(5);
            });
            pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                w.set_iso(false);
                w.set_ie(false);
                w.set_od(false);
                w.set_pue(false);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
            pac::SIO.gpio_out(0).value_clr().write_value(1 << pin);
        }
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
    /// Send the pixel buffer out of the pin with cycle-accurate WS2812 timing.
    pub fn transmit(&mut self) -> Result<u32, ()> {
        if !self.active { return Err(()); }
        let pin = self.pin;
        unsafe {
            pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                w.set_funcsel(5);
            });
            pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                w.set_iso(false);
                w.set_ie(false);
                w.set_od(false);
                w.set_pue(false);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
        }
        cortex_m::interrupt::free(|_| {
            for pixel in &self.pixels {
                let grb: u32 = ((pixel.g as u32) << 16) | ((pixel.r as u32) << 8) | (pixel.b as u32);
                for bit_idx in (0..24).rev() {
                    let bit = (grb >> bit_idx) & 1;
                    if bit == 1 {
                        unsafe { pac::SIO.gpio_out(0).value_set().write_value(1 << pin); }
                        cortex_m::asm::delay(35);
                        unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
                        cortex_m::asm::delay(20);
                    } else {
                        unsafe { pac::SIO.gpio_out(0).value_set().write_value(1 << pin); }
                        cortex_m::asm::delay(18);
                        unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
                        cortex_m::asm::delay(35);
                    }
                }
            }
            unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
            cortex_m::asm::delay(2500);
        });
        self.transmitted = self.transmitted.saturating_add(self.pixels.len() as u32);
        Ok(self.pixels.len() as u32)
    }
}

pub fn transmit_colors(colors: &[(u8, u8, u8)]) -> Result<u32, ()> {
    let pin = WS2812_PINS[0];
    unsafe {
        pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(false);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
    }
    cortex_m::interrupt::free(|_| {
        for &(r, g, b) in colors {
            let grb: u32 = ((g as u32) << 16) | ((r as u32) << 8) | (b as u32);
            for bit_idx in (0..24).rev() {
                let bit = (grb >> bit_idx) & 1;
                if bit == 1 {
                    unsafe { pac::SIO.gpio_out(0).value_set().write_value(1 << pin); }
                    cortex_m::asm::delay(35);
                    unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
                    cortex_m::asm::delay(20);
                } else {
                    unsafe { pac::SIO.gpio_out(0).value_set().write_value(1 << pin); }
                    cortex_m::asm::delay(18);
                    unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
                    cortex_m::asm::delay(35);
                }
            }
        }
        unsafe { pac::SIO.gpio_out(0).value_clr().write_value(1 << pin); }
        cortex_m::asm::delay(2500);
    });
    Ok(colors.len() as u32)
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
