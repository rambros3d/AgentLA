#![allow(dead_code)]
use core::sync::atomic::{AtomicU8, Ordering};

use crate::pin_owner::{self, ClaimError, Driver, SlotId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode { Idle = 0, I2cTiny = 1, Slcan = 2, I3c = 3, Uart = 4, NeoPixel = 5 }

static MODE: AtomicU8 = AtomicU8::new(0);

pub fn active() -> Mode {
    match MODE.load(Ordering::Relaxed) {
        1 => Mode::I2cTiny, 2 => Mode::Slcan, 3 => Mode::I3c,
        4 => Mode::Uart, 5 => Mode::NeoPixel, _ => Mode::Idle,
    }
}
pub fn set_active(m: Mode) { MODE.store(m as u8, Ordering::Relaxed); }

// Phase 4 submodules.
pub mod i2c;
pub mod slcan;
pub mod ws2812;
pub mod i3c;


// Safety-limited slot storage (interior via raw pointer to satisfy Rust 2024).
static mut I2C_SLOT: Option<SlotId> = None;
static mut SLCAN_SLOT: Option<SlotId> = None;
static mut LED_SLOT: Option<SlotId> = None;
static mut I3C_SLOT: Option<SlotId> = None;

macro_rules! slot_read { ($name:ident) => { unsafe { core::ptr::addr_of!($name).read() } } }
macro_rules! slot_write { ($name:ident, $val:expr) => { unsafe { core::ptr::addr_of_mut!($name).write($val) } } }

pub fn i2c_on() -> Result<(), &'static str> { i2c_enable().map_err(|_| "pin conflict") }
pub fn i2c_off() { i2c_disable() }
pub fn i2c_enable() -> Result<(), ClaimError> {
    unsafe {
        if slot_read!(I2C_SLOT).is_none() {
            let slot = pin_owner::claim(Driver::Adapter, &i2c::I2C_PINS, pin_owner::CAP_I2C)?;
            slot_write!(I2C_SLOT, Some(slot));
        }
    }
    set_active(Mode::I2cTiny);
    Ok(())
}
pub fn i2c_disable() {
    unsafe {
        if let Some(slot) = slot_read!(I2C_SLOT) {
            let _ = pin_owner::release(slot);
            slot_write!(I2C_SLOT, None);
        }
    }
    if active() == Mode::I2cTiny { set_active(Mode::Idle); }
}

pub fn slcan_on() -> Result<(), &'static str> { slcan_enable().map_err(|_| "pin conflict") }
pub fn slcan_off() { slcan_disable() }
pub fn slcan_enable() -> Result<(), ClaimError> {
    unsafe {
        if slot_read!(SLCAN_SLOT).is_none() {
            let slot = pin_owner::claim(Driver::Adapter, &slcan::SLCAN_PINS, pin_owner::CAP_PIO)?;
            slot_write!(SLCAN_SLOT, Some(slot));
        }
    }
    set_active(Mode::Slcan);
    Ok(())
}
pub fn slcan_disable() {
    unsafe {
        if let Some(slot) = slot_read!(SLCAN_SLOT) {
            let _ = pin_owner::release(slot);
            slot_write!(SLCAN_SLOT, None);
        }
    }
    if active() == Mode::Slcan { set_active(Mode::Idle); }
}

pub fn can_on() -> Result<(), &'static str> { slcan_enable().map_err(|_| "pin conflict") }
pub fn can_off() { slcan_disable() }
pub fn can_enable() -> Result<(), ClaimError> { slcan_enable() }
pub fn can_disable() { slcan_disable() }

pub fn led_on() -> Result<(), &'static str> { led_enable().map_err(|_| "pin conflict") }
pub fn led_off() { led_disable() }
pub fn led_enable() -> Result<(), ClaimError> {
    unsafe {
        if slot_read!(LED_SLOT).is_none() {
            let slot = pin_owner::claim(Driver::Adapter, &ws2812::WS2812_PINS, pin_owner::CAP_PIO)?;
            slot_write!(LED_SLOT, Some(slot));
        }
    }
    set_active(Mode::NeoPixel);
    Ok(())
}
pub fn led_disable() {
    unsafe {
        if let Some(slot) = slot_read!(LED_SLOT) {
            let _ = pin_owner::release(slot);
            slot_write!(LED_SLOT, None);
        }
    }
    if active() == Mode::NeoPixel { set_active(Mode::Idle); }
}
pub fn neopixel_enable() -> Result<(), ClaimError> { led_enable() }
pub fn neopixel_disable() { led_disable() }

pub fn i3c_on() -> Result<(), &'static str> { i3c_enable().map_err(|_| "pin conflict") }
pub fn i3c_off() { i3c_disable() }
pub fn i3c_enable() -> Result<(), ClaimError> {
    unsafe {
        if slot_read!(I3C_SLOT).is_none() {
            let slot = pin_owner::claim(Driver::Adapter, &i3c::I3C_PINS, pin_owner::CAP_PIO)?;
            slot_write!(I3C_SLOT, Some(slot));
        }
    }
    set_active(Mode::I3c);
    Ok(())
}
pub fn i3c_disable() {
    unsafe {
        if let Some(slot) = slot_read!(I3C_SLOT) {
            let _ = pin_owner::release(slot);
            slot_write!(I3C_SLOT, None);
        }
    }
    if active() == Mode::I3c { set_active(Mode::Idle); }
}

pub fn state_json() -> heapless::String<256> {
    let mode = active();
    let mode_str = match mode {
        Mode::Idle => "idle",
        Mode::I2cTiny => "i2c_tiny",
        Mode::Slcan => "slcan",
        Mode::I3c => "i3c",
        Mode::Uart => "uart",
        Mode::NeoPixel => "neopixel",
    };
    let i2c_claimed = slot_read!(I2C_SLOT).is_some();
    let slcan_claimed = slot_read!(SLCAN_SLOT).is_some();
    let led_claimed = slot_read!(LED_SLOT).is_some();
    let i3c_claimed = slot_read!(I3C_SLOT).is_some();
    let mut b = heapless::String::new();
    let _ = core::fmt::Write::write_fmt(&mut b, core::format_args!(
        r#"{{"active":"{}","i2c":{},"slcan":{},"led":{},"i3c":{}}}"#,
        mode_str, i2c_claimed, slcan_claimed, led_claimed, i3c_claimed));
    b
}
pub fn bridge_state_json() -> heapless::String<256> { state_json() }
