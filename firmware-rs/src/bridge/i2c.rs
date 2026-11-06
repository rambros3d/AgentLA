#![allow(dead_code, unused)]
//! I2C-tiny-USB / MCP2221 emulation and direct hardware I2C bus master.
//!
//! Provides open-drain bit-banged I2C on flexible GPIO pins with bus scanning,
//! register reads, and multi-byte writes.

use crate::board::ActiveBoard;
use crate::pin_owner;
use agentla_bsp::BoardProfile;
use rp_pac as pac;

pub const I2C_PINS: [u32; 2] = [
    ActiveBoard::ADAPTERS.default_i2c.sda,
    ActiveBoard::ADAPTERS.default_i2c.scl,
];

fn sda_high(sda_pin: u32) {
    unsafe {
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << sda_pin);
        pac::PADS_BANK0.gpio(sda_pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
    }
}

fn sda_low(sda_pin: u32) {
    unsafe {
        pac::SIO.gpio_out(0).value_clr().write_value(1 << sda_pin);
        pac::PADS_BANK0.gpio(sda_pin as usize).modify(|w| {
            w.set_od(false);
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << sda_pin);
    }
}

fn sda_read(sda_pin: u32) -> bool {
    (pac::SIO.gpio_in(0).read() & (1 << sda_pin)) != 0
}

fn scl_high(scl_pin: u32) {
    unsafe {
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << scl_pin);
        pac::PADS_BANK0.gpio(scl_pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
    }
}

fn scl_low(scl_pin: u32) {
    unsafe {
        pac::SIO.gpio_out(0).value_clr().write_value(1 << scl_pin);
        pac::PADS_BANK0.gpio(scl_pin as usize).modify(|w| {
            w.set_od(false);
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << scl_pin);
    }
}

fn i2c_delay() {
    // 100 kHz bit rate: half-period ~5 µs at 150 MHz
    cortex_m::asm::delay(250);
}

pub fn hw_i2c_init(sda_pin: u32, scl_pin: u32) {
    unsafe {
        for &pin in &[sda_pin, scl_pin] {
            pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                w.set_funcsel(5);
            });
            pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
        }
    }
}

pub fn hw_i2c_start(sda: u32, scl: u32) {
    sda_high(sda);
    scl_high(scl);
    i2c_delay();
    sda_low(sda);
    i2c_delay();
    scl_low(scl);
    i2c_delay();
}

pub fn hw_i2c_stop(sda: u32, scl: u32) {
    sda_low(sda);
    i2c_delay();
    scl_high(scl);
    i2c_delay();
    sda_high(sda);
    i2c_delay();
}

pub fn hw_i2c_write_byte(sda: u32, scl: u32, mut byte: u8) -> bool {
    for _ in 0..8 {
        if (byte & 0x80) != 0 {
            sda_high(sda);
        } else {
            sda_low(sda);
        }
        byte <<= 1;
        i2c_delay();
        scl_high(scl);
        i2c_delay();
        scl_low(scl);
        i2c_delay();
    }
    sda_high(sda);
    i2c_delay();
    scl_high(scl);
    i2c_delay();
    let ack = !sda_read(sda);
    scl_low(scl);
    i2c_delay();
    ack
}

pub fn hw_i2c_read_byte(sda: u32, scl: u32, ack: bool) -> u8 {
    let mut byte = 0u8;
    sda_high(sda);
    for _ in 0..8 {
        byte <<= 1;
        i2c_delay();
        scl_high(scl);
        i2c_delay();
        if sda_read(sda) {
            byte |= 1;
        }
        scl_low(scl);
        i2c_delay();
    }
    if ack {
        sda_low(sda);
    } else {
        sda_high(sda);
    }
    i2c_delay();
    scl_high(scl);
    i2c_delay();
    scl_low(scl);
    sda_high(sda);
    i2c_delay();
    byte
}

pub fn hw_i2c_scan(sda: u32, scl: u32) -> heapless::Vec<u8, 32> {
    let mut found = heapless::Vec::new();
    hw_i2c_init(sda, scl);
    for addr in 0x08..=0x77u8 {
        hw_i2c_start(sda, scl);
        let ack = hw_i2c_write_byte(sda, scl, addr << 1);
        hw_i2c_stop(sda, scl);
        if ack {
            let _ = found.push(addr);
        }
    }
    found
}

pub fn hw_i2c_read(sda: u32, scl: u32, addr: u8, reg: u8, dest: &mut [u8]) -> bool {
    hw_i2c_init(sda, scl);
    hw_i2c_start(sda, scl);
    if !hw_i2c_write_byte(sda, scl, addr << 1) {
        hw_i2c_stop(sda, scl);
        return false;
    }
    if !hw_i2c_write_byte(sda, scl, reg) {
        hw_i2c_stop(sda, scl);
        return false;
    }
    hw_i2c_start(sda, scl);
    if !hw_i2c_write_byte(sda, scl, (addr << 1) | 1) {
        hw_i2c_stop(sda, scl);
        return false;
    }
    let n = dest.len();
    for i in 0..n {
        let ack = i + 1 < n;
        dest[i] = hw_i2c_read_byte(sda, scl, ack);
    }
    hw_i2c_stop(sda, scl);
    true
}

pub fn hw_i2c_write(sda: u32, scl: u32, addr: u8, reg: u8, data: &[u8]) -> bool {
    hw_i2c_init(sda, scl);
    hw_i2c_start(sda, scl);
    if !hw_i2c_write_byte(sda, scl, addr << 1) {
        hw_i2c_stop(sda, scl);
        return false;
    }
    if !hw_i2c_write_byte(sda, scl, reg) {
        hw_i2c_stop(sda, scl);
        return false;
    }
    for &b in data {
        if !hw_i2c_write_byte(sda, scl, b) {
            hw_i2c_stop(sda, scl);
            return false;
        }
    }
    hw_i2c_stop(sda, scl);
    true
}

pub struct I2cBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub status: u8,
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
        hw_i2c_init(I2C_PINS[0], I2C_PINS[1]);
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
    pub fn process_command(&mut self, frame: &[u8]) -> u8 {
        if frame.is_empty() { return self.status; }
        match frame[0] {
            0x10 => {
                self.status = 0x20;
            }
            0x20 => {
                for &b in &frame[1..] { let _ = self.tx.push(b); }
                self.status = 0x21;
            }
            0x30 => {
                self.rx.extend_from_slice(&self.tx).ok();
                self.status = 0x31;
            }
            0x00 => { self.status = 0x10; }
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
        assert!(!b.active());
    }
}