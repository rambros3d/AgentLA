//! Pin ownership arbiter registry and conflict management.

use heapless::Vec;

use crate::quiesce::PinQuiesce;
use crate::types::{
    ClaimError, Driver, SlotId, CAP_ADC, CAP_I2C, CAP_PIO, CAP_PWM, CAP_SIO, CAP_SPI, CAP_UART,
};

/// Maximum simultaneous active claims supported by the registry.
pub const MAX_CLAIMS: usize = 16;

/// Maximum number of pins that can be grouped into a single claim.
pub const MAX_PINS: usize = 10;

/// Default flexible GPIO pool (capable of dynamic runtime claims).
pub const FLEX_PINS: [u32; 23] = [
    0, 1, 2, 3, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 26, 27, 28, 29,
];
pub const FLEX_PIN_COUNT: u32 = FLEX_PINS.len() as u32;

/// Default analog-capable GPIOs (subset of flexible pool).
pub const ANALOG_PINS: [u32; 4] = [26, 27, 28, 29];
pub const ANALOG_PIN_COUNT: u32 = ANALOG_PINS.len() as u32;

/// Default fixed programmer bridge pins (GPIO 4..9).
pub const FIXED_PINS: [u32; 6] = [4, 5, 6, 7, 8, 9];

/// Construct a 32-bit pin bitmask from a slice of pin numbers.
pub const fn mask_from_pins(pins: &[u32]) -> u32 {
    let mut mask = 0u32;
    let mut i = 0;
    while i < pins.len() {
        if pins[i] < 32 {
            mask |= 1u32 << pins[i];
        }
        i += 1;
    }
    mask
}

pub const DEFAULT_FLEX_MASK: u32 = mask_from_pins(&FLEX_PINS);
pub const DEFAULT_FIXED_MASK: u32 = mask_from_pins(&FIXED_PINS);

/// Base capability bitmask for RP2350 GPIO pin. SIO, PWM, SPI, UART, I2C, and PIO
/// are supported across all flexible pins; ADC is supported on pins 26..29.
pub const fn capabilities(pin: u32) -> u32 {
    if pin >= 32 {
        return 0;
    }
    let base = CAP_SIO | CAP_PWM | CAP_SPI | CAP_UART | CAP_I2C | CAP_PIO;
    let adc = if pin >= 26 && pin <= 29 { CAP_ADC } else { 0 };
    base | adc
}

/// Representation of an active or vacant claim slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Claim {
    pub active: bool,
    pub handle: SlotId,
    pub driver: Driver,
    pub pins: [u32; MAX_PINS],
    pub npins: usize,
}

impl Claim {
    pub const fn empty() -> Self {
        Self {
            active: false,
            handle: SlotId(0),
            driver: Driver::Programmer,
            pins: [0u32; MAX_PINS],
            npins: 0,
        }
    }
}

/// Fixed-capacity GPIO pin ownership arbiter.
#[derive(Clone, Debug)]
pub struct PinArbiter<Q: PinQuiesce = ()> {
    slots: [Claim; MAX_CLAIMS],
    next_handle: u32,
    flex_mask: u32,
    fixed_mask: u32,
    quiesce: Q,
}

impl PinArbiter<()> {
    /// Create a new pin arbiter with default masks and a no-op quiesce handler.
    pub const fn new() -> Self {
        Self::with_quiesce(())
    }
}

impl Default for PinArbiter<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Q: PinQuiesce> PinArbiter<Q> {
    /// Create a new pin arbiter with default masks and a custom quiesce handler.
    pub const fn with_quiesce(quiesce: Q) -> Self {
        Self::with_masks_and_quiesce(DEFAULT_FLEX_MASK, DEFAULT_FIXED_MASK, quiesce)
    }

    /// Create a new pin arbiter with explicit pin masks and a custom quiesce handler.
    pub const fn with_masks_and_quiesce(flex_mask: u32, fixed_mask: u32, quiesce: Q) -> Self {
        Self {
            slots: [Claim::empty(); MAX_CLAIMS],
            next_handle: 1,
            flex_mask,
            fixed_mask,
            quiesce,
        }
    }

    /// Set the flexible pin bitmask.
    pub fn set_flex_mask(&mut self, mask: u32) {
        self.flex_mask = mask;
    }

    /// Set flexible pins from a slice.
    pub fn set_flex_pins(&mut self, pins: &[u32]) {
        self.flex_mask = mask_from_pins(pins);
    }

    /// Set the fixed pin bitmask.
    pub fn set_fixed_mask(&mut self, mask: u32) {
        self.fixed_mask = mask;
    }

    /// Set fixed pins from a slice.
    pub fn set_fixed_pins(&mut self, pins: &[u32]) {
        self.fixed_mask = mask_from_pins(pins);
    }

    /// Replace the quiesce handler.
    pub fn set_quiesce(&mut self, quiesce: Q) {
        self.quiesce = quiesce;
    }

    /// Reference to the registered quiesce handler.
    pub fn quiesce(&self) -> &Q {
        &self.quiesce
    }

    /// Mutable reference to the registered quiesce handler.
    pub fn quiesce_mut(&mut self) -> &mut Q {
        &mut self.quiesce
    }

    /// Check if a pin is part of the flexible pool.
    #[inline]
    pub fn is_flex(&self, pin: u32) -> bool {
        pin < 32 && (self.flex_mask & (1 << pin)) != 0
    }

    /// Check if a pin is part of the fixed board infrastructure pool.
    #[inline]
    pub fn is_fixed(&self, pin: u32) -> bool {
        pin < 32 && (self.fixed_mask & (1 << pin)) != 0
    }

    /// Return the capability bitmask for a pin (0 if not in flexible pool).
    pub fn capabilities(&self, pin: u32) -> u32 {
        if self.is_flex(pin) {
            capabilities(pin)
        } else {
            0
        }
    }

    /// Query whether a pin has all required capabilities.
    #[inline]
    pub fn has_capability(&self, pin: u32, required_caps: u32) -> bool {
        (self.capabilities(pin) & required_caps) == required_caps
    }

    /// Return the driver currently claiming `pin`, if any.
    pub fn owner_of(&self, pin: u32) -> Option<Driver> {
        for slot in &self.slots {
            if slot.active && slot.pins[..slot.npins].contains(&pin) {
                return Some(slot.driver);
            }
        }
        None
    }

    /// Return the claim handle owning `pin`, if any.
    pub fn has_claim(&self, pin: u32) -> Option<SlotId> {
        for slot in &self.slots {
            if slot.active && slot.pins[..slot.npins].contains(&pin) {
                return Some(slot.handle);
            }
        }
        None
    }

    /// Return the list of pins assigned to a claim handle.
    pub fn handle_pins(&self, slot: SlotId) -> Option<Vec<u32, MAX_PINS>> {
        let claim = self.claim_of(slot)?;
        let mut v = Vec::new();
        v.extend_from_slice(&claim.pins[..claim.npins]).ok()?;
        Some(v)
    }

    /// Retrieve the claim record for an active handle.
    pub fn claim_of(&self, slot: SlotId) -> Option<Claim> {
        self.slots
            .iter()
            .find(|c| c.active && c.handle == slot)
            .copied()
    }

    /// Number of active claims in the registry.
    pub fn active_claims(&self) -> usize {
        self.slots.iter().filter(|s| s.active).count()
    }

    /// Atomically claim a set of pins for a driver requiring `required_caps`.
    ///
    /// All-or-nothing guarantee: On any failure (e.g. conflict, missing capability, out-of-pool),
    /// the registry is left completely unchanged and no pins are assigned.
    pub fn claim(
        &mut self,
        driver: Driver,
        pins: &[u32],
        required_caps: u32,
    ) -> Result<SlotId, ClaimError> {
        self.claim_inner(driver, pins, required_caps, false)
    }

    /// Atomically claim pins including fixed infrastructure pins (used during boot init).
    pub fn claim_fixed(
        &mut self,
        driver: Driver,
        pins: &[u32],
        required_caps: u32,
    ) -> Result<SlotId, ClaimError> {
        self.claim_inner(driver, pins, required_caps, true)
    }

    /// Internal claim transaction implementation.
    pub fn claim_inner(
        &mut self,
        driver: Driver,
        pins: &[u32],
        required_caps: u32,
        allow_fixed: bool,
    ) -> Result<SlotId, ClaimError> {
        if pins.is_empty() || pins.len() > MAX_PINS {
            return Err(ClaimError::NotInPool);
        }

        // Validation pass - no mutations permitted before full validation passes.
        for &pin in pins {
            if pin >= 32 {
                return Err(ClaimError::NotInPool);
            }
            if !self.is_flex(pin) {
                if allow_fixed && self.is_fixed(pin) {
                    // Allowed for fixed claims
                } else if self.is_fixed(pin) {
                    return Err(ClaimError::FixedPin);
                } else {
                    return Err(ClaimError::NotInPool);
                }
            }
            if self.owner_of(pin).is_some() {
                return Err(ClaimError::Conflict);
            }
            if !self.has_capability(pin, required_caps) {
                return Err(ClaimError::CapabilityMissing);
            }
        }

        let slot_idx = match self.slots.iter().position(|s| !s.active) {
            Some(idx) => idx,
            None => return Err(ClaimError::NoFreeSlots),
        };

        let handle = SlotId(self.next_handle);
        self.next_handle = if self.next_handle == u32::MAX {
            1
        } else {
            self.next_handle + 1
        };

        let mut pin_array = [0u32; MAX_PINS];
        pin_array[..pins.len()].copy_from_slice(pins);

        self.slots[slot_idx] = Claim {
            active: true,
            handle,
            driver,
            pins: pin_array,
            npins: pins.len(),
        };

        Ok(handle)
    }

    /// Release a claim handle, returning the slot to the free pool and invoking the
    /// registered [`PinQuiesce::quiesce_pin`] callback for every freed pin.
    ///
    /// Returns `true` if the handle was active and released, `false` if not found or already released.
    pub fn release(&mut self, slot: SlotId) -> bool {
        self.release_claim(slot).is_some()
    }

    /// Release a claim handle, invoking quiesce on each pin and returning the released `Claim`.
    pub fn release_claim(&mut self, slot: SlotId) -> Option<Claim> {
        let slot_idx = self
            .slots
            .iter()
            .position(|s| s.active && s.handle == slot)?;
        let claim = self.slots[slot_idx];
        self.slots[slot_idx].active = false;

        for &pin in &claim.pins[..claim.npins] {
            self.quiesce.quiesce_pin(pin);
        }

        Some(claim)
    }

    /// Drop every active claim except those belonging to `keep_driver` (e.g. `Driver::Programmer`),
    /// invoking quiesce callbacks on each freed pin and returning the number of released claims.
    pub fn clear_transient(&mut self, keep_driver: Driver) -> usize {
        let mut released = 0;
        for slot in &mut self.slots {
            if slot.active && slot.driver != keep_driver {
                slot.active = false;
                for &pin in &slot.pins[..slot.npins] {
                    self.quiesce.quiesce_pin(pin);
                }
                released += 1;
            }
        }
        released
    }
}
