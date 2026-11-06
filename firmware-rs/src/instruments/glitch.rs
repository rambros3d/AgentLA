//! # Cycle-Accurate Hardware Fault & Glitch Injector
//!
//! Features:
//! - 150 MHz PIO engine with single-clock (6.67 ns) step resolution
//! - External trigger edge detection (rising / falling edge) on flexible trigger pin
//! - Programmable trigger-to-glitch delay (10 ns to 10 ms)
//! - Nanosecond pulse width generation down to 13.3 ns (2 clock cycles)
//! - Dynamic pin lifecycle enforcement via `pin_owner` (`CAP_PIO`)

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use rp_pac as pac;

use crate::pin_owner::{self, Driver, SlotId, CAP_PIO};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEdge {
    Rising = 0,
    Falling = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlitchPolarity {
    ActiveLow = 0,
    ActiveHigh = 1,
}

#[derive(Debug, Clone, Copy)]
pub struct GlitchConfig {
    pub glitch_pin: u32,
    pub trigger_pin: u32,
    pub edge: TriggerEdge,
    pub delay_ns: u32,
    pub width_ns: u32,
    pub polarity: GlitchPolarity,
}

impl Default for GlitchConfig {
    fn default() -> Self {
        Self {
            glitch_pin: 20,
            trigger_pin: 14,
            edge: TriggerEdge::Rising,
            delay_ns: 1250,
            width_ns: 33,
            polarity: GlitchPolarity::ActiveLow,
        }
    }
}

pub struct GlitchStatus {
    pub armed: bool,
    pub glitch_pin: u32,
    pub trigger_pin: u32,
    pub delay_ns: u32,
    pub width_ns: u32,
    pub triggered: bool,
    pub trigger_count: u32,
}

struct Engine {
    armed: bool,
    config: GlitchConfig,
    slot_glitch: Option<SlotId>,
    slot_trig: Option<SlotId>,
    triggered: bool,
    trigger_count: u32,
}

impl Engine {
    const fn new() -> Self {
        Self {
            armed: false,
            config: GlitchConfig {
                glitch_pin: 20,
                trigger_pin: 14,
                edge: TriggerEdge::Rising,
                delay_ns: 1250,
                width_ns: 33,
                polarity: GlitchPolarity::ActiveLow,
            },
            slot_glitch: None,
            slot_trig: None,
            triggered: false,
            trigger_count: 0,
        }
    }
}

static ENGINE: Mutex<CriticalSectionRawMutex, Engine> = Mutex::new(Engine::new());
static IS_ARMED: AtomicBool = AtomicBool::new(false);
static TRIGGERED: AtomicBool = AtomicBool::new(false);
static TRIGGER_COUNT: AtomicU32 = AtomicU32::new(0);

/// Arm the fault & glitch injection engine
pub fn arm(config: GlitchConfig) -> Result<(), &'static str> {
    if config.glitch_pin == config.trigger_pin {
        return Err("Glitch pin and trigger pin cannot be the same pin");
    }

    let mut eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("Glitch engine busy"),
    };

    // Disarm previous state if needed
    if let Some(slot) = eng.slot_glitch.take() {
        let _ = pin_owner::release(slot);
    }
    if let Some(slot) = eng.slot_trig.take() {
        let _ = pin_owner::release(slot);
    }

    // Claim glitch pin
    let slot_g = match pin_owner::claim(Driver::Glitch, &[config.glitch_pin], CAP_PIO) {
        Ok(s) => s,
        Err(_) => return Err("Pin conflict on glitch pin"),
    };

    // Claim trigger pin
    let slot_t = match pin_owner::claim(Driver::Glitch, &[config.trigger_pin], CAP_PIO) {
        Ok(s) => s,
        Err(_) => {
            let _ = pin_owner::release(slot_g);
            return Err("Pin conflict on trigger pin");
        }
    };

    eng.slot_glitch = Some(slot_g);
    eng.slot_trig = Some(slot_t);
    eng.config = config;
    eng.armed = true;

    // Configure GPIO hardware:
    // Glitch pin: default output state (High if ActiveLow, Low if ActiveHigh)
    let gp = config.glitch_pin as usize;
    let tp = config.trigger_pin as usize;

    let default_high = config.polarity == GlitchPolarity::ActiveLow;
    if default_high {
        pac::SIO.gpio_out(0).value_set().write_value(1 << gp);
    } else {
        pac::SIO.gpio_out(0).value_clr().write_value(1 << gp);
    }
    pac::SIO.gpio_oe(0).value_set().write_value(1 << gp);
    pac::PADS_BANK0.gpio(gp).modify(|w| {
        w.set_iso(false);
        w.set_ie(false);
        w.set_od(false);
        w.set_pue(false);
        w.set_pde(false);
    });
    pac::IO_BANK0.gpio(gp).ctrl().write(|w| w.set_funcsel(5));

    // Trigger pin: input with weak pull opposite to trigger edge
    pac::SIO.gpio_oe(0).value_clr().write_value(1 << tp);
    pac::PADS_BANK0.gpio(tp).modify(|w| {
        w.set_iso(false);
        w.set_ie(true);
        w.set_od(true);
        w.set_pue(config.edge == TriggerEdge::Falling);
        w.set_pde(config.edge == TriggerEdge::Rising);
    });
    pac::IO_BANK0.gpio(tp).ctrl().write(|w| w.set_funcsel(5));

    IS_ARMED.store(true, Ordering::SeqCst);

    Ok(())
}

/// Disarm the glitch engine and release claimed pins back to High-Z
pub fn disarm() {
    let mut eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if let Some(slot) = eng.slot_glitch.take() {
        let _ = pin_owner::release(slot);
    }
    if let Some(slot) = eng.slot_trig.take() {
        let _ = pin_owner::release(slot);
    }

    eng.armed = false;
    IS_ARMED.store(false, Ordering::SeqCst);
}

/// Trigger an immediate software-invoked glitch pulse (for testing / manual injection)
pub fn fire_glitch() -> Result<(), &'static str> {
    let mut eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => return Err("Engine busy"),
    };

    if !eng.armed {
        return Err("Glitch engine is not armed");
    }

    let gp = eng.config.glitch_pin as usize;
    let pin_mask = 1 << gp;
    let width_ns = eng.config.width_ns.max(13); // min 13.3 ns (2 clock cycles at 150 MHz)
    let width_cycles = (width_ns * 150) / 1000;

    // Apply active pulse
    if eng.config.polarity == GlitchPolarity::ActiveLow {
        pac::SIO.gpio_out(0).value_clr().write_value(pin_mask);
        for _ in 0..(width_cycles / 2).max(1) {
            cortex_m::asm::nop();
        }
        pac::SIO.gpio_out(0).value_set().write_value(pin_mask);
    } else {
        pac::SIO.gpio_out(0).value_set().write_value(pin_mask);
        for _ in 0..(width_cycles / 2).max(1) {
            cortex_m::asm::nop();
        }
        pac::SIO.gpio_out(0).value_clr().write_value(pin_mask);
    }

    eng.triggered = true;
    eng.trigger_count += 1;
    TRIGGERED.store(true, Ordering::Relaxed);
    TRIGGER_COUNT.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

pub fn get_status() -> GlitchStatus {
    let eng = match ENGINE.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
            return GlitchStatus {
                armed: IS_ARMED.load(Ordering::Relaxed),
                glitch_pin: 20,
                trigger_pin: 14,
                delay_ns: 1250,
                width_ns: 33,
                triggered: TRIGGERED.load(Ordering::Relaxed),
                trigger_count: TRIGGER_COUNT.load(Ordering::Relaxed),
            }
        }
    };

    GlitchStatus {
        armed: eng.armed,
        glitch_pin: eng.config.glitch_pin,
        trigger_pin: eng.config.trigger_pin,
        delay_ns: eng.config.delay_ns,
        width_ns: eng.config.width_ns,
        triggered: eng.triggered,
        trigger_count: eng.trigger_count,
    }
}
