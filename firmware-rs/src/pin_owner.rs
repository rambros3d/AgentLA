#![allow(dead_code)] // Public surface is consumed by the servo engine and REST router.

//! # Pin Ownership Registry
//!
//! In-RAM GPIO claim arbiter (Rust embodiment of `pin-ownership`), with handles, capability
//! checks, all-or-nothing claim transactions and synchronous quiesce-to-safe-state on release.
//! Logical claim arbitration and capability enforcement are delegated to the standalone
//! `agentla-pin-arbiter` crate.
//!
//! ## Pin map (board-specific, binding per `specs/pin-ownership`)
//! - **Fixed** (always owned by `Driver::Programmer`, non-claimable): the programmer bridge
//!   block imported from [`crate::programmer::pins::BRIDGE_PINS`] (GPIO 4–14 minus 11).
//! - **Flexible**: GPIO 0–3, 11, 15–24, 26–29 (26–29 additionally analog-capable). GPIO 25
//!   (status LED) and GPIO 30/31 (USB pads) belong to no pool.
//! - **Capabilities**: every flexible pin supports SIO / PWM / SPI / UART / I2C / PIO0–2;
//!   GPIO 26–29 also support ADC.
//!
//! ## Quiesce policy
//! Release and blank-state take the register writes of `programmer/pins.rs` (pad isolation
//! off, input enable on, output disable, funcsel SIO, SIO direction in) but use *disabled*
//! pull resistors, matching the spec's blank state ("function SIO, direction input, pulls
//! disabled"). The programmer keeps its own pull-up tristate behaviour in `pins.rs`.

use core::sync::atomic::{AtomicBool, Ordering};

#[allow(unused_imports)]
pub use agentla_pin_arbiter::{
    capabilities as arbiter_capabilities, Claim, ClaimError, ClaimErrorKind, Driver, PinArbiter,
    PinQuiesce, SlotId, ANALOG_PINS, ANALOG_PIN_COUNT, CAP_ADC, CAP_I2C, CAP_PIO, CAP_PWM,
    CAP_SIO, CAP_SPI, CAP_UART, FLEX_PINS, FLEX_PIN_COUNT, MAX_CLAIMS, MAX_PINS,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use rp_pac as pac;

use crate::board::ActiveBoard;
use crate::programmer::pins::BRIDGE_PINS;
use agentla_bsp::BoardProfile;

// ---------------------------------------------------------------------------
// Pin map constants
// ---------------------------------------------------------------------------

/// Status LED pin — claimed by no flexible role.
pub const LED_PIN: u32 = match ActiveBoard::FEATURES.onboard_led {
    Some(pin) => pin,
    None => 25,
};

/// Hardware quiesce hook for RP2350 SIO pad register writes.
#[derive(Clone, Copy, Default)]
pub struct Rp2350Quiesce;

impl PinQuiesce for Rp2350Quiesce {
    fn quiesce_pin(&mut self, pin: u32) {
        quiesce_pin(pin);
    }
}

pub mod state {
    #[allow(unused_imports)]
    pub use agentla_pin_arbiter::*;
}

// ---------------------------------------------------------------------------
// Registry instance + module-level operations (IO layer)
// ---------------------------------------------------------------------------

pub static PIN_REGISTRY: Mutex<CriticalSectionRawMutex, PinArbiter<Rp2350Quiesce>> =
    Mutex::new(PinArbiter::with_quiesce(Rp2350Quiesce));

static INIT_DONE: AtomicBool = AtomicBool::new(false);

/// Boot-time initialization (task 1.3 + blank-start of `pin-ownership`):
/// 1. quiesces every flexible pin to SIO/input/no-pull, then
/// 2. registers the programmer bridge (imported from `programmer::pins`) as fixed
///    `Driver::Programmer` claims, so all later flexible claims get 409s on GPIO 4–14.
pub fn init() {
    if INIT_DONE.swap(true, Ordering::Relaxed) {
        return;
    }

    quiesce_flex_pool();

    let mut registry = PIN_REGISTRY
        .try_lock()
        .expect("pin registry uncontended at boot");
    let bridge: [u32; BRIDGE_PINS.len()] = core::array::from_fn(|i| BRIDGE_PINS[i] as u32);
    let handle = registry
        .claim_fixed(Driver::Programmer, &bridge, CAP_SIO)
        .expect("bridge pin count fits registry");
    defmt::info!(
        "Pin registry up: fixed programmer bridge (GPIO 4–14, handle {}) registered, \
         {} flexible pins blanked to SIO/in/no-pull",
        handle.0,
        FLEX_PIN_COUNT
    );
}

/// Claim a set of flexible pins. Atomic: on success every pin is registered and placed
/// in safe state (SIO funcsel, input direction, no pulls) for the new owner to configure;
/// on failure the registry is unchanged and no pin state is touched.
pub fn claim(driver: Driver, pins: &[u32], required_caps: u32) -> Result<SlotId, ClaimError> {
    let mut registry = PIN_REGISTRY
        .try_lock()
        .map_err(|_| ClaimError::Busy)?;
    let slot = registry.claim(driver, pins, required_caps)?;
    drop(registry);
    for &pin in pins {
        quiesce_pin(pin);
    }
    Ok(slot)
}

/// Release a claim by [`SlotId`]. Every freed pin is quiesced synchronously before this
/// returns. Idempotent: releasing an unknown or already-released handle performs no
/// hardware writes.
pub fn release(slot: SlotId) -> Result<(), ClaimError> {
    let released = {
        let mut registry = PIN_REGISTRY
            .try_lock()
            .map_err(|_| ClaimError::Busy)?;
        registry.release(slot)
    };
    if released {
        // Design decision 4: quiesce is synchronous register writes with a fixed settle
        // timer after the last write (rail discharge parity; no async waiting).
        embassy_time::block_for(embassy_time::Duration::from_millis(1));
    }
    Ok(())
}

/// Drop every non-`Programmer` claim and blank the whole flexible pool. This is the
/// profile-apply / emergency-safe-state entrypoint (pins quiesce, claims vanish; the
/// programmer bridge keeps its fixed ownership and its own pin policy in `pins.rs`).
pub fn quiesce_and_reset() {
    let released = {
        let mut registry = PIN_REGISTRY
            .try_lock()
            .expect("pin registry uncontended in quiesce_and_reset");
        registry.clear_transient(Driver::Programmer)
    };
    quiesce_flex_pool();
    // Design decision 4: fixed settle timer after the last quiesce write.
    embassy_time::block_for(embassy_time::Duration::from_millis(1));
    defmt::info!(
        "Pin quiesce+reset: freed {} transient claim(s), flexible pool blanked",
        released
    );
}

// ---------------------------------------------------------------------------
// Queries (consumed by the REST pins endpoints)
// ---------------------------------------------------------------------------

/// Owning handle of `pin`, if any.
pub fn has_claim(pin: u32) -> Option<SlotId> {
    PIN_REGISTRY.try_lock().ok()?.has_claim(pin)
}

/// Current driver holding `pin`, if any.
pub fn owner_of(pin: u32) -> Option<Driver> {
    PIN_REGISTRY.try_lock().ok()?.owner_of(pin)
}

/// Pins held by a claim handle (for AR responses and release auditing).
pub fn handle_pins(slot: SlotId) -> Option<heapless::Vec<u32, MAX_PINS>> {
    PIN_REGISTRY.try_lock().ok()?.handle_pins(slot)
}

/// `pin_owner_inventory_t` parity: flexible and analog-capable pin counts.
pub struct Inventory {
    pub flexible_pin_count: u32,
    pub analog_capable_pin_count: u32,
}

pub fn inventory() -> Inventory {
    Inventory {
        flexible_pin_count: FLEX_PIN_COUNT,
        analog_capable_pin_count: ANALOG_PIN_COUNT,
    }
}

pub const fn is_flex(pin: u32) -> bool {
    let mut i = 0;
    while i < FLEX_PINS.len() {
        if FLEX_PINS[i] == pin {
            return true;
        }
        i += 1;
    }
    false
}

pub const fn is_fixed(pin: u32) -> bool {
    let bridge = crate::programmer::pins::BRIDGE_PINS;
    let mut i = 0;
    while i < bridge.len() {
        if bridge[i] as u32 == pin {
            return true;
        }
        i += 1;
    }
    false
}

/// Capability bitmask of a pin (`0` for pins outside the flexible pool).
pub const fn capabilities(pin: u32) -> u32 {
    if is_flex(pin) {
        agentla_pin_arbiter::capabilities(pin)
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// IO layer — match `programmer/pins.rs` register write style incl. ISO-pad handling
// ---------------------------------------------------------------------------

/// Blank-state single pin: pad isolation off, input enabled, output disabled, pulls off,
/// funcsel SIO, SIO output-enable bit cleared. Spec blank state = SIO / in / no-pull.
pub(crate) fn quiesce_pin(pin: u32) {
    pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
        w.set_iso(false);
        w.set_ie(true);
        w.set_od(true); // output disabled = high-Z input
        w.set_pue(false);
        w.set_pde(false);
    });
    pac::SIO.gpio_oe(0).value_clr().write_value(1u32 << pin);
    pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
        w.set_funcsel(5); // SIO (SIOB_PROC_0) so gpio_in reads live voltages
    });
}

/// Quiesce every flexible pin (boot blank state & profile transitions).
pub(crate) fn quiesce_flex_pool() {
    for &pin in &FLEX_PINS {
        quiesce_pin(pin);
    }
}
