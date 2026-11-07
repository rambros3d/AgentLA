//! Multi-channel Servo and generic PWM integer timing arithmetic
//!
//! Provides 50 Hz servo fast-path, fixed-point fractional divisor search,
//! pulse-width microsecond clamping, and continuous throttle/angle mapping.

/// Default system clock frequency (150 MHz).
pub const SYS_CLK_HZ: u32 = 150_000_000;
/// REST/spec clamp for generic frequency path (1..=100,000 Hz).
pub const MAX_FREQ_HZ: u32 = 100_000;

/// Register values for one PWM slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SliceConf {
    pub div_int: u8,
    pub div_frac: u8,
    pub top: u16,
}

pub type ServoMath = SliceConf;

/// 50 Hz fast path: 1 µs effective tick at 150 MHz (`div 150`), 20 ms period
/// (`top 19999`), duty = pulse µs.
pub const SERVO_50HZ: SliceConf = SliceConf {
    div_int: 150,
    div_frac: 0,
    top: 19999,
};

/// Counter wrap counts `SYS·16 / (freq·(16·div_int + div_frac))`.
pub const fn wrap_counts(freq: u32, div_int: u8, div_frac: u8) -> u32 {
    wrap_counts_with_clk(SYS_CLK_HZ, freq, div_int, div_frac)
}

/// Counter wrap counts parameterized by system clock.
pub const fn wrap_counts_with_clk(sys_clk: u32, freq: u32, div_int: u8, div_frac: u8) -> u32 {
    let denom = freq as u64 * (16u64 * div_int as u64 + div_frac as u64);
    if denom == 0 {
        return 0;
    }
    ((sys_clk as u64 * 16) / denom) as u32
}

/// Slice configuration for `freq` Hz using default 150 MHz system clock.
/// Returns `None` only for `freq == 0`.
pub fn slice_conf(freq: u32) -> Option<SliceConf> {
    slice_conf_with_clk(SYS_CLK_HZ, freq)
}

/// Slice configuration for `freq` Hz given an arbitrary system clock.
pub fn slice_conf_with_clk(sys_clk: u32, freq: u32) -> Option<SliceConf> {
    if freq == 0 {
        return None;
    }
    if freq == 50 && sys_clk == SYS_CLK_HZ {
        return Some(SERVO_50HZ);
    }
    let freq = freq.clamp(1, MAX_FREQ_HZ);

    let units = (sys_clk / (freq * 4096)).max(16);
    let mut units = units;
    loop {
        let div_int = (units >> 4).min(255) as u8;
        let div_frac = (units & 0xF) as u8;
        let counts = wrap_counts_with_clk(sys_clk, freq, div_int, div_frac);
        if counts >= 2 && counts - 1 <= 65534 {
            return Some(SliceConf {
                div_int,
                div_frac,
                top: (counts - 1) as u16,
            });
        }
        if div_int >= 255 {
            // Divisor at maximum: wrap no longer fits; clamp it.
            let top = counts.saturating_sub(1).clamp(1, 65534) as u16;
            return Some(SliceConf {
                div_int,
                div_frac: 0,
                top,
            });
        }
        units += 1; // Slow divisor one unit step and retry
    }
}

/// Duty counts for `pulse_us` at `freq` Hz with wrap `top`:
/// `level = pulse_us * freq * (top + 1) / 1_000_000` clamped to `[0, top + 1]`.
pub fn level_counts(pulse_us: u32, freq: u32, top: u16) -> u16 {
    let level = (pulse_us as u64 * freq as u64 * (top as u64 + 1)) / 1_000_000;
    level.clamp(0, top as u64 + 1) as u16
}

/// Clamp pulse width to `[min_us, max_us]`.
pub fn clamp_pulse_us(pulse_us: u32, min_us: u32, max_us: u32) -> u32 {
    pulse_us.clamp(min_us, max_us)
}

/// Calculate pulse width in microseconds for a positional angle in degrees (0..=180).
pub fn angle_to_pulse(angle: u32, min_us: u32, max_us: u32) -> u32 {
    let angle = angle.min(180);
    min_us + (angle * (max_us - min_us)) / 180
}

/// Continuous-throttle pulse mapping:
/// `|t| <= 5` -> neutral 1500 µs;
/// `t >= 6` -> forward linear 1520..max_us;
/// `t <= -6` -> reverse linear 1480..min_us (`t = -100` -> min_us).
pub fn continuous_pulse(throttle: i32, min_us: u32, max_us: u32) -> u32 {
    const NEUTRAL: i64 = 1500;
    const BAND_LOW: i64 = 1480;
    const BAND_HIGH: i64 = 1520;
    let (min, max) = (min_us as i64, max_us as i64);
    if (-5..=5).contains(&throttle) {
        return NEUTRAL as u32;
    }
    let pulse = if throttle > 0 {
        let t = throttle - 5; // 1..=95
        BAND_HIGH + (max - BAND_HIGH) * t as i64 / 95
    } else {
        let t = -throttle - 5; // 1..=95
        BAND_LOW - (BAND_LOW - min) * t as i64 / 95
    };
    pulse.clamp(min, max) as u32
}
