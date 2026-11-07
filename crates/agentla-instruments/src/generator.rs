//! Signal generator PWM frequency divider and TOP wrap math
//!
//! Converts target output frequency and system clock into 8.4 fractional divider
//! and 16-bit TOP wrap values for hardware PWM slices.

pub const DEFAULT_SYS_CLK_HZ: u32 = 150_000_000;

/// PWM slice timing parameters (clock divisor and wrap TOP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PwmTiming {
    pub div_int: u8,
    pub div_frac: u8,
    pub top: u16,
}

impl PwmTiming {
    pub const fn new(div_int: u8, div_frac: u8, top: u16) -> Self {
        Self {
            div_int,
            div_frac,
            top,
        }
    }

    pub const fn into_tuple(self) -> (u8, u8, u16) {
        (self.div_int, self.div_frac, self.top)
    }
}

impl From<PwmTiming> for (u8, u8, u16) {
    fn from(t: PwmTiming) -> Self {
        t.into_tuple()
    }
}

pub type PwmTimingMath = PwmTiming;

/// Calculate PWM slice parameters for arbitrary frequency given a system clock.
///
/// Output: `(div_int, div_frac, top)` where divisor is `div_int + div_frac / 16`.
pub fn calculate_slice_params(sys_clk_hz: u32, freq_hz: u32) -> PwmTiming {
    let freq = freq_hz.clamp(1, 75_000_000);

    if freq >= 75_000_000 {
        return PwmTiming {
            div_int: 1,
            div_frac: 0,
            top: 1, // 75 MHz square wave
        };
    }

    if freq >= 100_000 {
        // Fast path for high frequencies (div = 1.0)
        let top = ((sys_clk_hz / freq).saturating_sub(1)).clamp(1, 65534) as u16;
        return PwmTiming {
            div_int: 1,
            div_frac: 0,
            top,
        };
    }

    // Generic path for 1 Hz .. 100 kHz with 8.4 fractional divider
    // Units of 1/16 divisor step
    let mut units = (sys_clk_hz / (freq * 4096)).max(16);
    let mut top: u64 = 65535;

    while units <= 4096 {
        let div_int = (units / 16).clamp(1, 255);
        let div_frac = (units % 16) as u8;
        let counts = (sys_clk_hz as u64 * 16) / ((freq as u64) * (units as u64));
        if counts > 0 && counts - 1 <= 65534 {
            top = counts - 1;
            return PwmTiming {
                div_int: div_int as u8,
                div_frac,
                top: top as u16,
            };
        }
        units += 1;
    }

    // Lowest frequencies: max divisor (255 + 15/16)
    PwmTiming {
        div_int: 255,
        div_frac: 15,
        top: top.min(65534) as u16,
    }
}

/// Convert percentage duty cycle (0..=100) into compare register count `level`.
pub fn duty_to_level(top: u16, duty_percent: u8) -> u16 {
    let duty_clamped = duty_percent.min(100);
    if duty_clamped == 0 {
        0u16
    } else if duty_clamped == 100 {
        top.saturating_add(1)
    } else {
        (((duty_clamped as u32) * ((top as u32) + 1)) / 100) as u16
    }
}
