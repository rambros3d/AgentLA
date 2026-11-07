//! Core types, driver roles, error representations, and hardware capabilities.

/// Bitmask flag for SIO (Software Input/Output / GPIO) capability.
pub const CAP_SIO: u32 = 1 << 0;
/// Bitmask flag for PWM slice capability.
pub const CAP_PWM: u32 = 1 << 1;
/// Bitmask flag for SPI peripheral muxing capability.
pub const CAP_SPI: u32 = 1 << 2;
/// Bitmask flag for UART peripheral muxing capability.
pub const CAP_UART: u32 = 1 << 3;
/// Bitmask flag for I2C peripheral muxing capability.
pub const CAP_I2C: u32 = 1 << 4;
/// Bitmask flag for PIO state machine muxing capability.
pub const CAP_PIO: u32 = 1 << 5;
/// Bitmask flag for ADC analog input capability.
pub const CAP_ADC: u32 = 1 << 6;

/// Opaque claim handle returned on successful reservation and required for release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SlotId(pub u32);

impl SlotId {
    /// Create a new slot handle with the given identifier.
    #[inline]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the raw integer identifier.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0
    }
}

impl core::fmt::Display for SlotId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u32> for SlotId {
    #[inline]
    fn from(val: u32) -> Self {
        Self(val)
    }
}

impl From<SlotId> for u32 {
    #[inline]
    fn from(slot: SlotId) -> Self {
        slot.0
    }
}

/// Peripheral driver role claiming hardware pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
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
    /// Numeric encoding for wire protocols and serialized profile payloads.
    #[inline]
    pub const fn enc(self) -> u8 {
        self as u8
    }

    /// Decode driver role from wire numeric representation.
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

    /// Parse driver role from REST / configuration identifier strings.
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

    /// Return the canonical string name of the driver role.
    pub const fn as_str(self) -> &'static str {
        match self {
            Driver::Programmer => "programmer",
            Driver::Servo => "servo",
            Driver::Gpio => "gpio",
            Driver::Adapter => "adapter",
            Driver::Capture => "capture",
            Driver::Oscilloscope => "oscilloscope",
            Driver::Generator => "generator",
            Driver::Ws2812 => "ws2812",
            Driver::Glitch => "glitch",
        }
    }
}

/// Errors returned during pin claim transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ClaimError {
    /// Pin is not part of the allocatable flexible pin pool.
    NotInPool,
    /// Pin is permanently assigned to fixed board infrastructure (e.g. programmer bridge).
    FixedPin,
    /// Pin lacks one or more requested hardware peripheral capabilities.
    CapabilityMissing,
    /// Pin is currently held by another active claim.
    Conflict,
    /// Alias for `Conflict` retained for backward compatibility.
    AlreadyClaimed,
    /// All available claim slots in the registry are in use.
    NoFreeSlots,
    /// Registry mutex / lock contention.
    Busy,
}

/// Backward compatibility alias for `ClaimError`.
pub type ClaimErrorKind = ClaimError;

impl core::fmt::Display for ClaimError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ClaimError::NotInPool => write!(f, "pin not in flexible pool"),
            ClaimError::FixedPin => write!(f, "pin is reserved for fixed board infrastructure"),
            ClaimError::CapabilityMissing => write!(f, "pin lacks requested capability"),
            ClaimError::Conflict | ClaimError::AlreadyClaimed => {
                write!(f, "pin already claimed by another driver")
            }
            ClaimError::NoFreeSlots => write!(f, "no free claim slots available in registry"),
            ClaimError::Busy => write!(f, "pin registry lock contended"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ClaimError {}
