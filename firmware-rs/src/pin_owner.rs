#![allow(dead_code)] // Public surface is consumed by the servo engine (task group 3) and the
                     // REST router (task group 4); it becomes reachable when those land.

//! # Pin Ownership Registry
//!
//! In-RAM GPIO claim arbiter (Rust embodiment of `pin-ownership`), with handles, capability
//! checks, all-or-nothing claim transactions and synchronous quiesce-to-safe-state on release.
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
//!
//! ## Consumption contracts (designed now, per tasks 3.x / 4.x)
//! - **`servo` engine (group 3)** — servo/GPIO_FUNC_PWM driver claims, one pin per channel:
//!   ```ignore
//!   let slot = pin_owner::claim(pin_owner::Driver::Servo, &[pin], pin_owner::CAP_PWM).unwrap();
//!   // ... configure PWM slice ...
//!   pin_owner::release(slot).unwrap(); // detach: quiesces the pin (SIO/in/no-pull)
//!   // Profile transitions / emergency safe state:
//!   pin_owner::quiesce_and_reset();
//!   ```
//! - **`web.rs` REST pins endpoints (group 4)** — claim-by-pins+role returning a handle,
//!   release-by-handle and presence queries:
//!   ```ignore
//!   pin_owner::claim(pin_owner::Driver::Gpio, &pins, pin_owner::CAP_SIO) // -> SlotId, 409 on conflict
//!   pin_owner::release(slot)                                            // idempotent
//!   pin_owner::has_claim(pin)                                           // Option<SlotId>
//!   pin_owner::handle_pins(slot)                                        // Option<heapless::Vec<u32, 10>>
//!   pin_owner::owner_of(pin)                                            // Option<Driver>
//!   pin_owner::Driver::from_name(role)                                  // parse REST "role" field
//!   ```

use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use rp_pac as pac;

use crate::programmer::pins::BRIDGE_PINS;

use crate::board::ActiveBoard;
use agentla_bsp::BoardProfile;

// ---------------------------------------------------------------------------
// Pin map constants
// ---------------------------------------------------------------------------

/// Status LED pin — claimed by no flexible role.
pub const LED_PIN: u32 = match ActiveBoard::FEATURES.onboard_led {
    Some(pin) => pin,
    None => 25,
};
/// Flexible pool (capable of dynamic claims); GPIO 26–29 are additionally analog-capable.
pub const FLEX_PINS: [u32; 23] = [
    0, 1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26, 27, 28, 29,
];
pub const FLEX_PIN_COUNT: u32 = FLEX_PINS.len() as u32;
/// Adjectival-capable members of the flexible pool: GPIO 26, 27, 28, 29.
pub const ANALOG_PINS: [u32; 4] = [26, 27, 28, 29];
pub const ANALOG_PIN_COUNT: u32 = ANALOG_PINS.len() as u32;

// Capability bitmask.
pub const CAP_SIO: u32 = 1 << 0;
pub const CAP_PWM: u32 = 1 << 1;
pub const CAP_SPI: u32 = 1 << 2;
pub const CAP_UART: u32 = 1 << 3;
pub const CAP_I2C: u32 = 1 << 4;
pub const CAP_PIO: u32 = 1 << 5; // any of PIO0–2
pub const CAP_ADC: u32 = 1 << 6;

/// Claim handle, opaque token returned by [`claim`] and required by [`release`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct SlotId(pub u32);

/// Claiming driver role. `Programmer` owns the fixed bridge; everything else claims
/// flexible pins through [`claim`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum Driver {
    Programmer = 0,
    Servo = 1,
    Gpio = 2,
    Adapter = 3,
    Capture = 4,
    Oscilloscope = 5,
    Generator = 6,
    Ws2812 = 7,
    Glitch = 8,
}

impl Driver {
    pub(crate) const fn enc(self) -> u8 {
        self as u8
    }

    /// Decode from wire encoding (REST `handle` bodies, future profile payloads).
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Driver::Programmer),
            1 => Some(Driver::Servo),
            2 => Some(Driver::Gpio),
            3 => Some(Driver::Adapter),
            4 => Some(Driver::Capture),
            5 => Some(Driver::Oscilloscope),
            6 => Some(Driver::Generator),
            7 => Some(Driver::Ws2812),
            8 => Some(Driver::Glitch),
            _ => None,
        }
    }

    /// Parse the REST `role` string (task 4.3). Unknown roles conflict-report cleanly.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "programmer" => Some(Driver::Programmer),
            "servo" => Some(Driver::Servo),
            "gpio" => Some(Driver::Gpio),
            "adapter" => Some(Driver::Adapter),
            "capture" => Some(Driver::Capture),
            "oscilloscope" | "scope" => Some(Driver::Oscilloscope),
            "generator" => Some(Driver::Generator),
            "ws2812" => Some(Driver::Ws2812),
            "glitch" => Some(Driver::Glitch),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure decision logic (no hardware access; unit-testable state machine)
// ---------------------------------------------------------------------------

pub(crate) mod state {
    //! Registry state machine, fully decoupled from the register/IO layer below.
    //! Hardware writes are performed only by the outer module; this module is pure data so
    //! claim/conflict/release logic can be unit-tested without a target (task 1.4).

    use super::{
        CAP_ADC, CAP_I2C, CAP_PIO, CAP_PWM, CAP_SIO, CAP_SPI, CAP_UART, FLEX_PINS,
    };

    /// Maximum simultaneous claims (incl. the fixed `Programmer` block at init).
    pub const MAX_CLAIMS: usize = 16;
    /// Maximum pins per claim (the programmer bridge block itself is 10 pins).
    pub const MAX_PINS: usize = 10;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
    pub enum Kind {
        /// Pin is not in the flexible pool (USB pads, LED, unknown).
        NotInPool,
        /// Pin belongs to the programmer bridge block (GPIO 4–14 minus 11).
        FixedPin,
        /// Requested capability not supported by this pin.
        CapabilityMissing,
        /// Pin already claimed by driver `by` (this is the REST 409 case).
        AlreadyClaimed,
        /// All [`MAX_CLAIMS`] slots busy.
        NoFreeSlots,
        /// Registry mutex contended (should not occur under the cooperative scheduler).
        Busy,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
    pub struct ClaimError {
        pub kind: Kind,
        /// The offending pin, when the error is pin-specific (0 otherwise).
        pub pin: u32,
        /// Encoded driver currently holding `pin`, for `Kind::AlreadyClaimed` (0 otherwise).
        pub by: u8,
    }

    #[derive(Clone, Copy)]
    pub struct Claim {
        pub active: bool,
        pub handle: u32,
        pub driver: u8,
        pub pins: [u32; MAX_PINS],
        pub npins: usize,
    }

    #[derive(Clone, Copy)]
    pub struct Registry {
        slots: [Claim; MAX_CLAIMS],
        next_handle: u32,
    }

    impl Registry {
        pub const fn new() -> Self {
            Self {
                slots: [Claim {
                    active: false,
                    handle: 0,
                    driver: 0,
                    pins: [0u32; MAX_PINS],
                    npins: 0,
                }; MAX_CLAIMS],
                next_handle: 1,
            }
        }

        /// Atomic, all-or-nothing claim of `pins` for `driver` requiring `required_caps`.
        /// On any per-pin failure the registry is left completely untouched.
        pub fn claim(
            &mut self,
            driver: u8,
            pins: &[u32],
            required_caps: u32,
            allow_fixed: bool,
        ) -> Result<u32, ClaimError> {
            let err = |kind, pin, by| ClaimError { kind, pin, by };

            if pins.is_empty() || pins.len() > MAX_PINS {
                return Err(err(Kind::NotInPool, 0, 0));
            }

            // Validation pass — no mutation on any failure.
            for &pin in pins {
                if !Self::is_flex(pin) {
                    if allow_fixed && Self::is_fixed(pin) {
                        continue;
                    }
                    return Err(err(
                        if Self::is_fixed(pin) { Kind::FixedPin } else { Kind::NotInPool },
                        pin,
                        0,
                    ));
                }
                if let Some(by) = self.owner_driver(pin) {
                    return Err(err(Kind::AlreadyClaimed, pin, by));
                }
                if capabilities(pin) & required_caps != required_caps {
                    return Err(err(Kind::CapabilityMissing, pin, 0));
                }
            }

            let index = match self.slots.iter().position(|c| !c.active) {
                Some(i) => i,
                None => return Err(err(Kind::NoFreeSlots, 0, 0)),
            };

            // Commit.
            let handle = self.next_handle;
            self.next_handle = if handle == u32::MAX { 1 } else { handle + 1 };
            let slot = &mut self.slots[index];
            *slot = Claim {
                active: true,
                handle,
                driver,
                pins: [0u32; MAX_PINS],
                npins: pins.len(),
            };
            slot.pins[..pins.len()].copy_from_slice(pins);
            Ok(handle)
        }

        /// Release a claim by handle; returns `(driver, pin count, pins)` when the handle
        /// was active so the IO layer can quiesce the freed pins. Returns `None` for an
        /// unknown/expired handle — callers treat this as an idempotent no-op.
        pub fn release(&mut self, handle: u32) -> Option<(u8, usize, [u32; MAX_PINS])> {
            let index = self
                .slots
                .iter()
                .position(|c| c.active && c.handle == handle)?;
            let claim = self.slots[index];
            self.slots[index].active = false;
            Some((claim.driver, claim.npins, claim.pins))
        }

        /// Every transient claim (all drivers except `keep_driver`) is dropped; count only.
        pub fn clear_transient(&mut self, keep_driver: u8) -> usize {
            let mut released = 0;
            for slot in &mut self.slots {
                if slot.active && slot.driver != keep_driver {
                    slot.active = false;
                    released += 1;
                }
            }
            released
        }

        pub fn owner(&self, pin: u32) -> Option<(u32, u8)> {
            for slot in &self.slots {
                if slot.active && slot.pins[..slot.npins].contains(&pin) {
                    return Some((slot.handle, slot.driver));
                }
            }
            None
        }

        pub fn owner_driver(&self, pin: u32) -> Option<u8> {
            self.owner(pin).map(|(_, d)| d)
        }

        pub fn claim_of(&self, handle: u32) -> Option<Claim> {
            self.slots
                .iter()
                .find(|c| c.active && c.handle == handle)
                .copied()
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
    }

    /// Capability bitmask of a flexible pin. SIO is implied for every pin; 26–29 add ADC.
    /// (PWM/SPI/UART/I2C/PIO muxing is valid on every flexible GPIO on RP2350.)
    pub const fn capabilities(pin: u32) -> u32 {
        let base = CAP_SIO | CAP_PWM | CAP_SPI | CAP_UART | CAP_I2C | CAP_PIO;
        let adc = if pin >= 26 && pin <= 29 { CAP_ADC } else { 0 };
        base | adc
    }
}

pub use state::{ClaimError, Kind as ClaimErrorKind};

// ---------------------------------------------------------------------------
// Registry instance + module-level operations (IO layer)
// ---------------------------------------------------------------------------

pub static PIN_REGISTRY: Mutex<CriticalSectionRawMutex, state::Registry> =
    Mutex::new(state::Registry::new());

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
        .claim(Driver::Programmer.enc(), &bridge, CAP_SIO, true)
        .expect("bridge pin count fits registry");
    defmt::info!(
        "Pin registry up: fixed programmer bridge (GPIO 4–14, handle {}) registered, \
         {} flexible pins blanked to SIO/in/no-pull",
        handle,
        FLEX_PIN_COUNT
    );
}

/// Claim a set of flexible pins. Atomic: on success every pin is registered and placed
/// in safe state (SIO funcsel, input direction, no pulls) for the new owner to configure;
/// on failure the registry is unchanged and no pin state is touched.
pub fn claim(driver: Driver, pins: &[u32], required_caps: u32) -> Result<SlotId, ClaimError> {
    let mut registry = PIN_REGISTRY
        .try_lock()
        .map_err(|_| ClaimError { kind: ClaimErrorKind::Busy, pin: 0, by: 0 })?;
    let handle = registry.claim(driver.enc(), pins, required_caps, false)?;
    drop(registry);
    for &pin in pins {
        quiesce_pin(pin);
    }
    Ok(SlotId(handle))
}

/// Release a claim by [`SlotId`]. Every freed pin is quiesced synchronously before this
/// returns. Idempotent: releasing an unknown or already-released handle performs no
/// hardware writes.
pub fn release(slot: SlotId) -> Result<(), ClaimError> {
    let freed = {
        let mut registry = PIN_REGISTRY
            .try_lock()
            .map_err(|_| ClaimError { kind: ClaimErrorKind::Busy, pin: 0, by: 0 })?;
        registry.release(slot.0)
    };
    if let Some((_driver, npins, pins)) = freed {
        for &pin in &pins[..npins] {
            quiesce_pin(pin);
        }
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
        registry.clear_transient(Driver::Programmer.enc())
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
// Queries (task 1.4 helpers, consumed by the REST pins endpoints)
// ---------------------------------------------------------------------------

/// Owning handle of `pin`, if any.
pub fn has_claim(pin: u32) -> Option<SlotId> {
    PIN_REGISTRY
        .try_lock()
        .ok()?
        .owner(pin)
        .map(|(handle, _)| SlotId(handle))
}

/// Current driver holding `pin`, if any.
pub fn owner_of(pin: u32) -> Option<Driver> {
    let encoded = PIN_REGISTRY
        .try_lock()
        .ok()?
        .owner(pin)
        .map(|(_, d)| d)?;
    Driver::from_u8(encoded)
}

/// Pins held by a claim handle (for AR responses and release auditing).
pub fn handle_pins(slot: SlotId) -> Option<heapless::Vec<u32, { state::MAX_PINS }>> {
    let claim = PIN_REGISTRY.try_lock().ok()?.claim_of(slot.0)?;
    let mut pins = heapless::Vec::new();
    pins.extend_from_slice(&claim.pins[..claim.npins]).ok()?;
    Some(pins)
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
    state::Registry::is_flex(pin)
}

pub const fn is_fixed(pin: u32) -> bool {
    state::Registry::is_fixed(pin)
}

/// Capability bitmask of a pin (`0` for pins outside the flexible pool).
pub const fn capabilities(pin: u32) -> u32 {
    if state::Registry::is_flex(pin) {
        state::capabilities(pin)
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

// ---------------------------------------------------------------------------
// Unit tests (host-only: `cargo test` cannot build the full no_std firmware crate,
// see task 1.4 — the state machine itself is hardware-independent; on-target behaviour
// regression runs under the REST hardware round, task 5.3).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::state::{self, Kind, Registry};
    use super::{CAP_ADC, CAP_SIO};

    #[test]
    fn fixed_and_out_of_pool_pins_rejected() {
        let mut registry = Registry::new();
        for &pin in &crate::programmer::pins::BRIDGE_PINS {
            let err = registry.claim(1, &[pin as u32], CAP_SIO, false).unwrap_err();
            assert_eq!(err.kind, Kind::FixedPin);
            assert_eq!(err.pin, pin as u32);
        }
        for pin in [25u32, 30, 31] {
            let err = registry.claim(1, &[pin], 0, false).unwrap_err();
            assert_eq!(err.kind, Kind::NotInPool);
        }
        // Nothing had been claimed anywhere.
        for &pin in &super::FLEX_PINS {
            assert_eq!(registry.owner(pin), None);
        }
    }

    #[test]
    fn claim_conflict_preserves_state() {
        let mut registry = Registry::new();
        let first = registry.claim(1, &[17], 0, false).unwrap();
        let err = registry.claim(2, &[17], 0, false).unwrap_err();
        assert_eq!(err.kind, Kind::AlreadyClaimed);
        assert_eq!(err.pin, 17);
        assert_eq!(err.by, 1);
        assert_eq!(registry.owner(17), Some((first, 1)));
        assert!(registry.claim_of(first).unwrap().active);
    }

    #[test]
    fn atomic_all_or_nothing() {
        let mut registry = Registry::new();
        let first = registry.claim(2, &[20], 0, false).unwrap();
        let err = registry.claim(1, &[19, 20], 0, false).unwrap_err();
        assert_eq!(err.kind, Kind::AlreadyClaimed);
        // Pin 19 must NOT be claimed although pin 20 conflicted.
        assert_eq!(registry.owner(19), None);
        assert_eq!(registry.owner(20), Some((first, 2)));
    }

    #[test]
    fn failure_leaves_registry_untouched() {
        let mut registry = Registry::new();
        // Pin 17 lacks ADC; 27 has it. Missing capability on the first pin of the claim.
        let err = registry
            .claim(1, &[17, 27], CAP_ADC, false)
            .unwrap_err();
        assert_eq!(err.kind, Kind::CapabilityMissing);
        assert_eq!(err.pin, 17);
        assert_eq!(registry.owner(17), None);
        assert_eq!(registry.owner(27), None);
        assert!(registry.claim(1, &[27], CAP_ADC, false).is_ok());
    }

    #[test]
    fn release_is_idempotent() {
        let mut registry = Registry::new();
        let handle = registry.claim(1, &[16], 0, false).unwrap();
        let (driver, npins, pins) = registry.release(handle).expect("claim alive");
        assert_eq!((driver, npins), (1, 1));
        assert_eq!(pins[0], 16);
        // Repeated release: no-op None (caller skips hardware writes).
        assert!(registry.release(handle).is_none());
        assert!(registry.release(424242).is_none());
        assert_eq!(registry.owner(16), None);
    }

    #[test]
    fn registry_capacity_and_handle_progression() {
        let mut registry = Registry::new();
        let mut last_handle = 0u32;
        for slot_index in 0..state::MAX_CLAIMS {
            let handle = registry
                .claim(1, &[FLEX_FOR_TEST[slot_index]], 0, false)
                .unwrap();
            assert!(handle > last_handle);
            last_handle = handle;
        }
        // No free slots left.
        assert_eq!(
            registry.claim(9, &[23], 0, false).unwrap_err().kind,
            Kind::NoFreeSlots
        );
        // Releasing one claim frees a slot; handles still increase.
        assert!(registry.release(1).is_some() || registry.release(2).is_some());
        assert!(registry.claim(1, &[23], 0, false).is_ok());
    }

    const FLEX_FOR_TEST: [u32; 16] = [
        0, 1, 2, 3, 11, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26,
    ];
}
