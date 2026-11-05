use core::sync::atomic::{AtomicU8, Ordering};

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
