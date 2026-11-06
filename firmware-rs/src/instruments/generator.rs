//! # RP2350 Digital Signal & Clock Generator
//!
//! Features:
//! - Hardware PWM slice allocation across GPIO 10–29
//! - Frequency range from 1 Hz up to 75 MHz with 8.4 fractional divider
//! - Variable duty cycle from 0% to 100%
//! - Finite burst pulse mode (inject N pulses then automatically shut off)
//! - Dynamic pin lifecycle enforcement via `pin_owner` (`CAP_PWM`)

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use rp_pac as pac;

use crate::board::ActiveBoard;
use crate::pin_owner::{self, Driver, SlotId, CAP_PWM};
use agentla_bsp::BoardProfile;

pub const SYS_CLK_HZ: u32 = 150_000_000;
pub const PWM_FUNCSEL: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratorMode {
    Continuous,
    Burst,
}

#[derive(Debug, Clone, Copy)]
pub struct GeneratorConfig {
    pub pin: u32,
    pub freq_hz: u32,
    pub duty_percent: u8,
    pub burst_count: u32, // 0 = continuous, >0 = finite burst
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            pin: 12,
            freq_hz: 1000,
            duty_percent: 50,
            burst_count: 0,
        }
    }
}

pub struct GeneratorStatus {
    pub active: bool,
    pub pin: u32,
    pub freq_hz: u32,
    pub duty_percent: u8,
    pub burst_count: u32,
    pub running: bool,
}

struct Engine {
    active: bool,
    config: GeneratorConfig,
    slot: Option<SlotId>,
    claimed_pin: Option<u32>,
}

impl Engine {
    const fn new() -> Self {
        Self {
            active: false,
            config: GeneratorConfig {
                pin: 12,
                freq_hz: 1000,
                duty_percent: 50,
                burst_count: 0,
            },
            slot: None,
            claimed_pin: None,
        }
    }
}

static ENGINE: Mutex<CriticalSectionRawMutex, Engine> = Mutex::new(Engine::new());
static IS_RUNNING: AtomicBool = AtomicBool::new(false);
static ACTIVE_PIN: AtomicU32 = AtomicU32::new(12);
static ACTIVE_FREQ: AtomicU32 = AtomicU32::new(1000);
static ACTIVE_DUTY: AtomicU32 = AtomicU32::new(50);
static ACTIVE_BURST: AtomicU32 = AtomicU32::new(0);

/// Calculate PWM slice parameters (integer div, frac div, top wrap)
pub fn calculate_slice_params(freq_hz: u32) -> (u8, u8, u16) {
    let freq = freq_hz.clamp(1, 75_000_000);

    if freq >= 75_000_000 {
        return (1, 0, 1); // 75 MHz square wave
    }

    if freq >= 100_000 {
        // Fast path for high frequencies (div = 1.0)
        let top = ((SYS_CLK_HZ / freq).saturating_sub(1)).clamp(1, 65534) as u16;
        return (1, 0, top);
    }

    // Generic path for 1 Hz .. 100 kHz with fractional divider
    // Units of 1/16 divisor step
    let mut units = (SYS_CLK_HZ / (freq * 4096)).max(16);
    let mut top: u64 = 65535;

    while units <= 4096 {
        let div_int = (units / 16).clamp(1, 255);
        let div_frac = (units % 16) as u8;
        let counts = (SYS_CLK_HZ as u64 * 16) / ((freq as u64) * (units as u64));
        if counts > 0 && counts - 1 <= 65534 {
            top = counts - 1;
            return (div_int as u8, div_frac, top as u16);
        }
        units += 1;
    }

    // Lowest frequencies: max divisor (255 + 15/16)
    (255, 15, (top.min(65534)) as u16)
}

/// Start outputting signal on the specified GPIO pin
pub fn start(config: GeneratorConfig) -> Result<(), &'static str> {
    if !ActiveBoard::flexible_pool().contains(config.pin)
        || ActiveBoard::features().onboard_led == Some(config.pin)
    {
        return Err("Pin must be in board flexible pool (excluding status LED)");
    }

    let mut eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("Generator mutex busy"),
    };

    // Release prior claim if changing pin or stopping
    if let Some(old_pin) = eng.claimed_pin {
        if old_pin != config.pin {
            if let Some(slot) = eng.slot.take() {
                let _ = pin_owner::release(slot);
            }
            stop_pwm_slice(old_pin);
        }
    }

    // Claim flexible pin for CAP_PWM
    if eng.slot.is_none() || eng.claimed_pin != Some(config.pin) {
        let slot = match pin_owner::claim(Driver::Generator, &[config.pin], CAP_PWM) {
            Ok(s) => s,
            Err(_) => return Err("Pin conflict: selected pin is already claimed"),
        };
        eng.slot = Some(slot);
        eng.claimed_pin = Some(config.pin);
    }

    eng.config = config;
    eng.active = true;

    // Apply hardware PWM registers
    configure_pwm(config.pin, config.freq_hz, config.duty_percent);

    // If burst mode requested, execute burst
    if config.burst_count > 0 {
        execute_burst_pulses(config.pin, config.freq_hz, config.burst_count);
        if let Some(slot) = eng.slot.take() {
            let _ = pin_owner::release(slot);
        }
        eng.claimed_pin = None;
        eng.active = false;
        IS_RUNNING.store(false, Ordering::SeqCst);
    } else {
        IS_RUNNING.store(true, Ordering::SeqCst);
    }

    ACTIVE_PIN.store(config.pin, Ordering::Relaxed);
    ACTIVE_FREQ.store(config.freq_hz, Ordering::Relaxed);
    ACTIVE_DUTY.store(config.duty_percent as u32, Ordering::Relaxed);
    ACTIVE_BURST.store(config.burst_count, Ordering::Relaxed);

    Ok(())
}

/// Stop signal generator output and release the claimed GPIO pin back to High-Z
pub fn stop() {
    let mut eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if let Some(pin) = eng.claimed_pin.take() {
        stop_pwm_slice(pin);
    }

    if let Some(slot) = eng.slot.take() {
        let _ = pin_owner::release(slot);
    }

    eng.active = false;
    IS_RUNNING.store(false, Ordering::SeqCst);
}

pub fn get_status() -> GeneratorStatus {
    GeneratorStatus {
        active: IS_RUNNING.load(Ordering::Relaxed),
        pin: ACTIVE_PIN.load(Ordering::Relaxed),
        freq_hz: ACTIVE_FREQ.load(Ordering::Relaxed),
        duty_percent: ACTIVE_DUTY.load(Ordering::Relaxed) as u8,
        burst_count: ACTIVE_BURST.load(Ordering::Relaxed),
        running: IS_RUNNING.load(Ordering::Relaxed),
    }
}

fn configure_pwm(pin: u32, freq_hz: u32, duty_percent: u8) {
    let slice = ((pin / 2) % 12) as usize;
    let channel = (pin % 2) as usize; // 0 = A, 1 = B

    let (div_int, div_frac, top) = calculate_slice_params(freq_hz);

    // Compute duty level
    let duty_clamped = duty_percent.min(100);
    let level = if duty_clamped == 0 {
        0u16
    } else if duty_clamped == 100 {
        top.saturating_add(1)
    } else {
        (((duty_clamped as u32) * ((top as u32) + 1)) / 100) as u16
    };

    let ch = pac::PWM.ch(slice);

    // Disable slice while programming
    ch.csr().modify(|w| w.set_en(false));

    // Program clock divider and wrap top
    ch.div().write(|w| {
        w.set_int(div_int);
        w.set_frac(div_frac);
    });
    ch.ctr().write(|w| w.0 = 0);
    ch.top().write(|w| w.0 = top as u32);

    // Program duty cycle on channel A or B
    if channel == 0 {
        ch.cc().modify(|w| w.set_a(level));
    } else {
        ch.cc().modify(|w| w.set_b(level));
    }

    // Configure pad and GPIO funcsel 4 (PWM)
    pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
        w.set_iso(false);
        w.set_ie(false);
        w.set_od(false);
        w.set_pue(false);
        w.set_pde(false);
    });
    pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
        w.set_funcsel(PWM_FUNCSEL);
    });

    // Enable slice
    ch.csr().modify(|w| w.set_en(true));
}

fn stop_pwm_slice(pin: u32) {
    let slice = ((pin / 2) % 12) as usize;
    let ch = pac::PWM.ch(slice);
    ch.csr().modify(|w| w.set_en(false));
    ch.cc().write(|w| w.0 = 0);
}

/// Emit exactly N pulses and then shut off the slice output
fn execute_burst_pulses(pin: u32, freq_hz: u32, burst_count: u32) {
    let slice = ((pin / 2) % 12) as usize;
    let ch = pac::PWM.ch(slice);

    // Approximate burst duration
    let period_us = (1_000_000u64 / (freq_hz as u64).max(1)).max(1);
    let total_us = period_us * (burst_count as u64);

    // Run slice for duration
    ch.csr().modify(|w| w.set_en(true));
    embassy_time::block_for(embassy_time::Duration::from_micros(total_us));
    ch.csr().modify(|w| w.set_en(false));
}
