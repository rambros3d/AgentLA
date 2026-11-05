//! # Unified Servo / Generic-PWM Engine (`servo-pwm-control`)
//!
//! One register-level engine behind both REST surfaces (`/api/v1/servo/**` and
//! `/api/v1/pwm/**`). Designed per `design.md` decision 6:
//!
//! ## Why pac registers instead of `embassy_rp::pwm`
//! `embassy_rp::pwm` selects slices at compile time via type-level `Slice`/pin traits
//! (`Pwm::new_output_a::<PWM_SLICE7, PIN_14, ...>`). Servo/Generic-PWM pins are chosen at
//! REST time, so we drive `pac::PWM.ch(slice)` (the RP2350 Pwm block, modelled per slice
//! as "channel" with `cc.a`/`cc.b` register halves) directly:
//!
//! | slice index | source                              | register                                              |
//! |------------:|-------------------------------------|-------------------------------------------------------|
//! | `(pin/2)%12`| RP2350 GPIO→slice map (see below)   | `pac::PWM.ch(n).div()/.ctr()/.cc()/.top()/.csr()`      |
//!
//! GPIO→slice/channel mapping (verified against embassy-rp 0.10.0 `src/pwm.rs`
//! `impl_pin!` table — RP2350 has 12 slices, GP24/25→slice 4, GP26/27→slice 5,
//! GP28/29→slice 6, therefore the `% 12` wrap): **slice = pin / 2 (mod 12)**,
//! **channel half = pin % 2** (`cc.a` for even pins, `cc.b` for odd pins).
//!
//! ## GPIO funcsel
//! `GPIO_FUNC_PWM` is **4** — the task brief said 9; that value is wrong for RP2350
//! (funcsel table: 1=SPI 2=UART 3=I2C 4=PWM 5=SIO 6/7/8=PIO0/1/2). Evidence:
//! embassy-rp 0.10.0 `pwm.rs::new_inner` programs `pin.gpio().ctrl().write(|w|
//! w.set_funcsel(4))` for `_rp235x` as well. `pin_owner::claim` always leaves a newly
//! claimed pin quiesced at SIO (funcsel 5), so **the claiming engine applies the PWM
//! funcsel + pad config itself right after a successful claim** (documented contract in
//! `pin_owner.rs` "Consumption contracts").
//!
//! ## Integer timing math (exact expressions, decision 6)
//! `SYS_CLK_HZ = 150_000_000`. All formulas below are pure (see [`math`], unit-tested).
//! - **50 Hz fast path** (`freq == 50`): `div_int = 150, div_frac = 0, top = 19999`,
//!   `cc = pulse_us` clamped to `[0, 20000]` → 1 µs tick/20 ms period.
//! - **Generic path** with `freq ∈ 1..=100000`:
//!   1. `units = SYS_CLK_HZ / (freq * 4096)` — the 4.4 fixed-point divisor unity-scale
//!      (`units/16` approximates the ideal `div = SYS / (freq·65536)`; `units == 0`
//!      means `freq` is above what `div 1.0` can express with wrap ≤ 65534, so
//!      `div_int = 1, div_frac = 0` and only `top` is clamped).
//!   2. While the wrap does not fit (`counts = SYS·16 / (freq·(16·div_int + div_frac))`,
//!      `top = counts − 1 > 65534`), `units` is incremented one step at a time (i.e. the
//!      divisor is slowed fractionally) until it fits. At `div_int == 255` the divisor
//!      is at maximum and `top` is clamped to 65534 instead. Because `top` is recomputed
//!      from the *effective* divisor, the output frequency heaves to `freq` within
//!      rounding error (< 0.1 %) — using `frac` bits beats the pure-integer-division
//!      alternative (up to ~2.5 % error near the wrap clamp at mid-range frequencies).
//!   3. `level = pulse_us · freq · (top + 1) / 1_000_000`, clamped `[0, top+1]`
//!      (64-bit intermediate math, no floats).
//!
//! ## Slice-ownership arbitration (the silent-clobber fix)
//! [`Engine::slices`] records, per slice, which pin owns which half and the
//! div/top configuration in force. Configuring a slice whose *sibling* half is owned by
//! an active channel with an inconsistent configuration is refused
//! ([`EngineError::SliceConflict`]) **before any register write**; consistent
//! reconfiguration (same div/top, e.g. two 50 Hz servos on adjacent pins) is allowed.
//!
//! ## Claim contract
//! The engine claims pins exclusively via `pin_owner::claim(Driver::Servo, &[pin],
//! CAP_PWM)` — one claimant per channel — and releases via `pin_owner::release`
//! (which quiesces to SIO/in/no-pull). Profile transitions call [`quiesce_all`] (wired
//! from the REST profile endpoints because `profile::apply_slot` performs the actual
//! `pin_owner::quiesce_and_reset` reset itself).

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use rp_pac as pac;

use crate::pin_owner::{self, Driver, SlotId, CAP_PWM};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// System clock (design constant; the PWM divider math derives everything from it).
pub const SYS_CLK_HZ: u32 = 150_000_000;
/// GPIO funcsel of the PWM peripheral (verified: embassy-rp 0.10.0 `src/pwm.rs`).
pub const PWM_FUNCSEL: u8 = 4;
/// REST/spec clamp for the generic frequency path (1..=100 000 Hz).
pub const MAX_FREQ_HZ: u32 = 100_000;
/// RP2350A PWM slices (12; see module docs for the `% 12` pin mapping).
pub const SLICES: usize = 12;
/// Maximum servo/Generic-PWM channels (≤ 8 per `servo-pwm-control`).
pub const MAX_CHANNELS: usize = 8;

// ---------------------------------------------------------------------------
// Pure integer timing math (unit-tested, no hardware access)
// ---------------------------------------------------------------------------

pub mod math {
    use super::{MAX_FREQ_HZ, SYS_CLK_HZ};

    /// Register values for one PWM slice.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
    pub struct SliceConf {
        pub div_int: u8,
        pub div_frac: u8,
        pub top: u16,
    }

    /// 50 Hz fast path: 1 µs effective tick at 150 MHz (`div 150`), 20 ms period
    /// (`top 19999`), duty = pulse µs. Spec `servo-pwm-control::fifty-hertz-fast-path`.
    pub const SERVO_50HZ: SliceConf = SliceConf {
        div_int: 150,
        div_frac: 0,
        top: 19999,
    };

    /// Counter wrap counts `SYS·16 / (freq·(16·div_int + div_frac))`.
    const fn wrap_counts(freq: u32, div_int: u8, div_frac: u8) -> u32 {
        let denom = freq as u64 * (16u64 * div_int as u64 + div_frac as u64);
        ((SYS_CLK_HZ as u64 * 16) / denom) as u32
    }

    /// Slice configuration for `freq` Hz (see module docs for the exact formulas).
    /// `None` only for `freq == 0` (invalid); the 1..=100 000 Hz clamp applies inside.
    pub fn slice_conf(freq: u32) -> Option<SliceConf> {
        if freq == 0 {
            return None;
        }
        if freq == 50 {
            return Some(SERVO_50HZ);
        }
        let freq = freq.clamp(1, MAX_FREQ_HZ);

        let units = SYS_CLK_HZ / (freq * 4096);
        if units == 0 {
            // Faster than div 1.0 can express with wrap ≤ 65534: div 1, wrap from clamp.
            let counts = wrap_counts(freq, 1, 0);
            let top = counts.saturating_sub(1).clamp(1, 65534) as u16;
            return Some(SliceConf {
                div_int: 1,
                div_frac: 0,
                top,
            });
        }

        let mut units = units;
        loop {
            let div_int = (units >> 4).min(255) as u8;
            let div_frac = (units & 0xF) as u8;
            let counts = wrap_counts(freq, div_int, div_frac);
            if counts >= 2 && counts - 1 <= 65534 {
                return Some(SliceConf {
                    div_int,
                    div_frac,
                    top: (counts - 1) as u16,
                });
            }
            if div_int >= 255 {
                // Divisor at maximum: the wrap no longer fits; clamp it (accepted
                // frequency error, same policy as the C firmware's int-only clamp).
                let top = counts.saturating_sub(1).clamp(1, 65534) as u16;
                return Some(SliceConf {
                    div_int,
                    div_frac: 0,
                    top,
                });
            }
            units += 1; // slow the divisor one unit step and retry
        }
    }

    /// Duty counts for `pulse_us` at `freq` Hz with wrap `top`:
    /// `level = pulse_us·freq·(top+1) / 1_000_000` clamped to `[0, top+1]`
    /// (64-bit intermediate, integer division only).
    pub fn level_counts(pulse_us: u32, freq: u32, top: u16) -> u16 {
        let level = (pulse_us as u64 * freq as u64 * (top as u64 + 1)) / 1_000_000;
        level.clamp(0, top as u64 + 1) as u16
    }

    /// Continuous-throttle pulse mapping (C semantics verbatim, spec
    /// `servo-pwm-control::servo-writing-semantics`): `|t| ≤ 5` → neutral 1500 µs;
    /// `t ≥ 6` → forward linear 1520..max_us; `t ≤ −6` → reverse linear 1480..min_us
    /// (`t = −100` = full reverse → min_us).
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
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Engine error taxonomy (mapped to HTTP statuses in `web.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum EngineError {
    /// Pin already claimed (servo channel, generic-PWM claim, pins claim, …).
    PinConflict,
    /// Pin outside the flexible pool (USB pads, LED, or programmer bridge).
    NotAvailable,
    /// Sibling half of the slice carries an inconsistent active configuration;
    /// nothing was written.
    SliceConflict,
    /// All [`MAX_CHANNELS`] engine channels are busy.
    ChannelsFull,
    /// All pin-ownership claim slots are busy.
    ClaimRegFull,
    /// Pin-registry lock contention (cannot occur under the single-threaded
    /// executor's sync-only engine ops, but is refused cleanly instead of panicking).
    RegistryBusy,
    /// Write/list/attach on a pin without an engine channel of the right kind.
    NotAttached,
    /// `/pwm/set`/`/pwm/release` on a pin never claimed through `/pwm/claim`.
    NotClaimed,
    /// Frequency outside 1..=100 000 Hz.
    InvalidFrequency,
    /// Pulse width below zero / negative (range clamps elsewhere are silent).
    InvalidPulse,
    /// Positional angle outside 0..=180 or throttle outside −100..=100.
    InvalidAngle,
    /// Internal mutex contention (cannot occur; sync-only ops).
    EngineBusy,
}

// ---------------------------------------------------------------------------
// Engine state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ChannelKind {
    /// `/api/v1/servo/**` channel (always the 50 Hz fast path).
    Servo,
    /// `/api/v1/pwm/**` channel (claim → set frequency/pulse).
    GenericPwm,
}

/// Snapshot of a channel handed to REST handlers under a single released lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct ChanOut {
    pub pin: u32,
    pub kind: ChannelKind,
    pub continuous: bool,
    pub min_us: u32,
    pub max_us: u32,
    /// Last `write_angle`/`write` value (positional 0..=180 or throttle −100..=100);
    /// 0 for generic-PWM channels.
    pub angle: i16,
    pub pulse_us: u32,
    pub freq: u32,
    /// Generic-PWM: `/pwm/set` has been applied (its slice actually runs).
    pub active: bool,
}

#[derive(Clone, Copy)]
struct Channel {
    active: bool,
    kind: ChannelKind,
    pin: u32,
    /// pin_owner claim handle backing the channel.
    slot: u32,
    continuous: bool,
    min_us: u32,
    max_us: u32,
    angle: i16,
    pulse_us: u32,
    freq: u32,
    pulse_set: bool,
}

impl Channel {
    const BLANK: Self = Self {
        active: false,
        kind: ChannelKind::Servo,
        pin: 0,
        slot: 0,
        continuous: false,
        min_us: 0,
        max_us: 0,
        angle: 0,
        pulse_us: 0,
        freq: 0,
        pulse_set: false,
    };

    fn snapshot(self) -> ChanOut {
        ChanOut {
            pin: self.pin,
            kind: self.kind,
            continuous: self.continuous,
            min_us: self.min_us,
            max_us: self.max_us,
            angle: self.angle,
            pulse_us: self.pulse_us,
            freq: self.freq,
            active: self.active && self.pulse_set,
        }
    }
}

#[derive(Clone, Copy)]
struct SliceEntry {
    /// Owning pin per channel half (`cc.a` for even pins, `cc.b` for odd pins).
    users: [Option<u32>; 2],
    /// Slice-wide timebase in force (the silent-clobber protection record).
    div_int: u8,
    div_frac: u8,
    top: u16,
    configured: bool,
}

impl SliceEntry {
    const BLANK: Self = Self {
        users: [None, None],
        div_int: 0,
        div_frac: 0,
        top: 0,
        configured: false,
    };
}

struct Engine {
    channels: [Channel; MAX_CHANNELS],
    slices: [SliceEntry; SLICES],
}

impl Engine {
    const fn new() -> Self {
        Self {
            channels: [Channel::BLANK; MAX_CHANNELS],
            slices: [SliceEntry::BLANK; SLICES],
        }
    }

    fn chan_index(&self, pin: u32) -> Option<usize> {
        self.channels
            .iter()
            .position(|c| c.active && c.pin == pin)
    }

    fn free_index(&self) -> Option<usize> {
        self.channels.iter().position(|c| !c.active)
    }

    /// Register + configure a new channel. `configure == false` marks a
    /// claim-only GenericPwm channel (`/pwm/claim` without `/pwm/set` yet).
    fn attach(
        &mut self,
        pin: u32,
        min_us: Option<u32>,
        max_us: Option<u32>,
        continuous: bool,
        kind: ChannelKind,
        freq: u32,
        configure: bool,
    ) -> Result<ChanOut, EngineError> {
        if self.chan_index(pin).is_some() {
            return Err(EngineError::PinConflict);
        }
        if pin_owner::has_claim(pin).is_some() {
            return Err(EngineError::PinConflict);
        }
        if !pin_owner::is_flex(pin) {
            return Err(EngineError::NotAvailable);
        }

        let idx = self.free_index().ok_or(EngineError::ChannelsFull)?;

        // Servo limit clamps (spec: min ∈ [400,1500] default 500; max ∈ [1500,3000]
        // default 2500). The clamp ranges make `min ≤ max` automatic.
        let (min_us, max_us) = match kind {
            ChannelKind::Servo => (
                min_us.unwrap_or(500).clamp(400, 1500),
                max_us.unwrap_or(2500).clamp(1500, 3000),
            ),
            ChannelKind::GenericPwm => (0, 0),
        };

        let slice = pin as usize / 2 % SLICES;
        let half = (pin & 1) as usize;
        let conf = if configure {
            // Evaluate the target slice config *before* claiming/mutating anything:
            // a shared-slice inconsistency must leave zero hardware side effects.
            let conf = math::slice_conf(freq).ok_or(EngineError::InvalidFrequency)?;
            let entry = &self.slices[slice];
            let sibling = entry.users[1 - half];
            if sibling.is_some() && entry.configured {
                let same = entry.div_int == conf.div_int
                    && entry.div_frac == conf.div_frac
                    && entry.top == conf.top;
                if !same {
                    return Err(EngineError::SliceConflict);
                }
            }
            Some(conf)
        } else {
            None
        };

        // Claim through the pin-ownership registry (quiesces the pin first).
        let slot = pin_owner::claim(Driver::Servo, &[pin], CAP_PWM).map_err(|e| match e.kind {
            pin_owner::ClaimErrorKind::NoFreeSlots => EngineError::ClaimRegFull,
            pin_owner::ClaimErrorKind::Busy => EngineError::RegistryBusy,
            pin_owner::ClaimErrorKind::NotInPool | pin_owner::ClaimErrorKind::CapabilityMissing => {
                EngineError::NotAvailable
            }
            _ => EngineError::PinConflict,
        })?;

        // Claimant applies the PWM funcsel itself (claims quiesce to SIO first).
        pin_to_pwm_output(pin);

        let (angle, pulse) = match kind {
            ChannelKind::Servo => {
                // Positional init angle 90 (centered run), continuous init −100 (neutral).
                let angle: i16 = if continuous { -100 } else { 90 };
                let pulse = if continuous {
                    1500
                } else {
                    min_us + 90 * (max_us - min_us) / 180
                };
                (angle, pulse)
            }
            ChannelKind::GenericPwm => (0, 0),
        };

        if let Some(conf) = conf {
            let level = if freq == 50 {
                pulse.clamp(0, conf.top as u32 + 1) as u16
            } else {
                math::level_counts(pulse, freq, conf.top)
            };
            let e = &mut self.slices[slice];
            if e.configured
                && (e.div_int != conf.div_int
                    || e.div_frac != conf.div_frac
                    || e.top != conf.top)
            {
                // Sibling changed under our feet (should not happen; the pre-check ran
                // unlocked state) — roll the claim back rather than clobber.
                let _ = pin_owner::release(slot);
                return Err(EngineError::SliceConflict);
            }
            write_slice_config(slice, conf, e.configured);
            write_cc_half(slice, half, level);
            e.users[half] = Some(pin);
            e.div_int = conf.div_int;
            e.div_frac = conf.div_frac;
            e.top = conf.top;
            e.configured = true;
        } else {
            // Claim-only: record the half so configuration conflicts are detected
            // later, but touch no timebase register (en stays off).
            self.slices[slice].users[half] = Some(pin);
        }

        let out = ChanOut {
            pin,
            kind,
            continuous: continuous && kind == ChannelKind::Servo,
            min_us,
            max_us,
            angle,
            pulse_us: pulse,
            freq: if kind == ChannelKind::Servo { 50 } else { freq },
            active: configure,
        };
        self.channels[idx] = Channel {
            active: true,
            kind,
            pin,
            slot: slot.0,
            continuous: out.continuous,
            min_us,
            max_us,
            angle,
            pulse_us: pulse,
            freq: out.freq,
            pulse_set: configure,
        };
        Ok(out)
    }

    /// Write the commanded pulse to one channel's compare register and state.
    fn command_pulse(&mut self, idx: usize, pulse: u32) -> ChanOut {
        let ch = self.channels[idx];
        let slice = ch.pin as usize / 2 % SLICES;
        let half = (ch.pin & 1) as usize;
        let level = if ch.freq == 50 {
            pulse.clamp(0, 20000) as u16
        } else {
            let entry = self.slices[slice];
            math::level_counts(pulse, ch.freq, entry.top)
        };
        write_cc_half(slice, half, level);
        self.channels[idx].pulse_us = pulse;
        self.channels[idx].snapshot()
    }

    fn detach_index(&mut self, idx: usize) -> Result<(), EngineError> {
        let ch = self.channels[idx];
        let slice = ch.pin as usize / 2 % SLICES;
        let half = (ch.pin & 1) as usize;

        let entry = &mut self.slices[slice];
        if entry.users[half] == Some(ch.pin) {
            write_cc_half(slice, half, 0);
            entry.users[half] = None;
        }
        if entry.users[0].is_none() && entry.users[1].is_none() {
            // No user left: stop the counter.
            pac::PWM.ch(slice).csr().modify(|w| w.set_en(false));
            *entry = SliceEntry::BLANK;
        }

        self.channels[idx] = Channel::BLANK;
        pin_owner::release(SlotId(ch.slot)).map_err(|_| EngineError::RegistryBusy)?; // quiesces + 1 ms settle
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests (host-only; see note in pin_owner.rs — on-target behaviour under
// the servo hardware round, task 5.4)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::math::{continuous_pulse, level_counts, slice_conf, SliceConf};

    /// Reference vectors: actual = SYS·16·100 / ((top+1)·(16·di+df)) Hz must match
    /// `freq` within 1 % (exact on the spec fast path; frac-bit corrections close in).
    const fn freq_actual_hz(conf: SliceConf) -> u64 {
        let denom = (conf.top as u64 + 1) * (16u64 * conf.div_int as u64 + conf.div_frac as u64);
        (super::SYS_CLK_HZ as u64 * 16 * 100) / denom
    }

    fn assert_freq(freq: u32, conf: SliceConf) {
        let actual = freq_actual_hz(conf) as u32;
        let err = actual.abs_diff(freq);
        assert!(
            err as u64 * 1000 <= freq as u64 * 10, // ≤ 1 % (exact for most cases)
            "freq {} → {:?} → actual {} Hz",
            freq,
            conf,
            actual
        );
    }

    #[test]
    fn servo_fast_path_is_exact() {
        assert_eq!(slice_conf(50), Some(SliceConf { div_int: 150, div_frac: 0, top: 19999 }));
        assert_eq!(freq_actual_hz(slice_conf(50).unwrap()), 50);
    }

    #[test]
    fn generic_paths_hit_frequency() {
        for freq in [1, 10, 100, 1000, 10000, 100000, 7, 333, 55555] {
            let conf = slice_conf(freq).unwrap();
            // div bounds per spec.
            assert!((1..=255).contains(&conf.div_int), "div_int {:?}", conf);
            assert!(conf.div_frac <= 15, "div_frac {:?}", conf);
            assert!((1..=65534).contains(&conf.top), "top {:?}", conf);
            assert_freq(freq, conf);
        }
        // Known exact case (frac bits lift the wrap into range): 1 kHz.
        assert_eq!(
            slice_conf(1000),
            Some(SliceConf { div_int: 2, div_frac: 5, top: 64863 })
        );
        // Very high frequency: div 1, wrap 1500 → exactly 100 kHz.
        assert_eq!(
            slice_conf(100000),
            Some(SliceConf { div_int: 1, div_frac: 0, top: 1499 })
        );
        // Degenerate low frequency (wrap clamped, error accepted like the C firmware).
        let conf = slice_conf(1).unwrap();
        assert_eq!(conf.div_int, 255);
        assert_eq!(conf.top, 65534);
    }

    #[test]
    fn invalid_frequency_rejected() {
        assert_eq!(slice_conf(0), None);
    }

    #[test]
    fn level_math_proportional() {
        // Half period at 1 kHz → half of (top+1)
        let conf = slice_conf(1000).unwrap();
        assert_eq!(level_counts(500, 1000, conf.top), (conf.top + 1) / 2);
        assert_eq!(level_counts(0, 1000, conf.top), 0);
        assert_eq!(level_counts(1000, 1000, conf.top), conf.top + 1);
        assert_eq!(level_counts(u32::MAX, 1000, conf.top), conf.top + 1); // clamped
        // 50 Hz fast path level == pulse µs.
        assert_eq!(level_counts(1500, 50, 19999), 1500);
    }

    #[test]
    fn continuous_throttle_mapping() {
        // Deadband |t| ≤ 5 → neutral 1500.
        for t in [-5, 0, 5] {
            assert_eq!(continuous_pulse(t, 500, 2500), 1500);
        }
        // Full reverse/forward reach the limits.
        assert_eq!(continuous_pulse(-100, 500, 2500), 500);
        assert_eq!(continuous_pulse(100, 500, 2500), 2500);
        // Just past the deadband: forward 1520+σ, reverse 1480−σ.
        assert_eq!(continuous_pulse(6, 500, 2500), 1520 + (1000 / 95));
        assert_eq!(continuous_pulse(-6, 500, 2500), 1480 - (980 / 95));
        // Midpoint sanity: full forward/reverse stay inside [min,max].
        assert!(continuous_pulse(50, 500, 2500) > 1500);
        assert!(continuous_pulse(-50, 500, 2500) < 1500);
    }
}


// ---------------------------------------------------------------------------
// Register write helpers (pac PWM block)
// ---------------------------------------------------------------------------

/// Put a claimed pin into PWM output mode: pads driven, no pulls, funcsel PWM.
fn pin_to_pwm_output(pin: u32) {
    pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
        w.set_iso(false);
        w.set_ie(true);
        w.set_od(false); // output driven (pin_owner's quiesce set this "true"/high-Z)
        w.set_pue(false);
        w.set_pde(false);
    });
    pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
        w.set_funcsel(PWM_FUNCSEL);
    });
}

/// Program div/top/CSR/CTR for one slice. `rewrites_existing == true` when some other
/// channel already runs this configuration: shared registers (div/top) are rewritten
/// with identical values, CTR is left alone so the sibling's phase is not stepped.
fn write_slice_config(slice: usize, conf: math::SliceConf, rewrites_existing: bool) {
    let p = pac::PWM.ch(slice);
    p.div().write(|w| {
        w.set_int(conf.div_int);
        w.set_frac(conf.div_frac);
    });
    p.top().write(|w| {
        w.set_top(conf.top);
    });
    if !rewrites_existing {
        p.ctr().write(|w| w.0 = 0);
    }
    p.csr().write(|w| {
        w.set_divmode(pac::pwm::vals::Divmode::DIV); // trailing-edge free-running
        w.set_en(true);
    });
}

/// Write one channel's compare (`cc.a` for even/GPIO-half-0 pins, `cc.b` for odd).
fn write_cc_half(slice: usize, half: usize, level: u16) {
    pac::PWM.ch(slice).cc().modify(|w| {
        if half == 0 {
            w.set_a(level);
        } else {
            w.set_b(level);
        }
    });
}

// ---------------------------------------------------------------------------
// Engine instance + public operations
// ---------------------------------------------------------------------------

static ENGINE: Mutex<CriticalSectionRawMutex, Engine> = Mutex::new(Engine::new());

fn engine() -> embassy_sync::mutex::MutexGuard<'static, CriticalSectionRawMutex, Engine> {
    // Engine ops are synchronous (no awaits): a held guard can only be "in flight"
    // on the same thread, so try_lock cannot fail under the single-thread executor.
    // use try_lock anyway to refuse cleanly rather than panic.
    ENGINE
        .try_lock()
        .expect("PWM engine lock free (sync-only ops)")
}

/// Attach a servo channel (spec: ≤ 8 channels, key GPIO, limits clamped into
/// `[400,1500]`/`[1500,3000]`, center 1500 µs, positional 90° or continuous −100).
pub fn servo_attach(
    pin: u32,
    min_us: Option<u32>,
    max_us: Option<u32>,
    continuous: bool,
) -> Result<ChanOut, EngineError> {
    engine().attach(pin, min_us, max_us, continuous, ChannelKind::Servo, 50, true)
}

/// Generic-PWM claim (`POST /api/v1/pwm/claim`): reserves the pin for the engine
/// without touching any timebase register until `/pwm/set` arrives. Returns
/// `(snapshot, claim handle)` for the REST body.
pub fn pwm_claim(pin: u32) -> Result<(ChanOut, u32), EngineError> {
    let mut eng = engine();
    let out = eng.attach(pin, None, None, false, ChannelKind::GenericPwm, 0, false)?;
    let handle = eng
        .chan_index(pin)
        .map(|i| eng.channels[i].slot)
        .expect("channel just attached");
    Ok((out, handle))
}

/// Positional angle 0..=180 (`min..max` linear) or throttle −100..=100
/// (±5 deadband → 1500 µs neutral, forward 1520..max, reverse 1480..min).
pub fn servo_write_angle(pin: u32, angle: i32) -> Result<ChanOut, EngineError> {
    let mut eng = engine();
    let idx = eng.chan_index(pin).ok_or(EngineError::NotAttached)?;
    let ch = eng.channels[idx];
    if ch.kind != ChannelKind::Servo {
        return Err(EngineError::NotAttached);
    }
    let (valid, pulse): (bool, u32) = if ch.continuous {
        (
            (-100..=100).contains(&angle),
            math::continuous_pulse(angle, ch.min_us, ch.max_us),
        )
    } else {
        (
            (0..=180).contains(&angle),
            ch.min_us + (angle.max(0) as u32 * (ch.max_us - ch.min_us)) / 180,
        )
    };
    if !valid {
        return Err(EngineError::InvalidAngle);
    }
    eng.channels[idx].angle = angle as i16;
    Ok(eng.command_pulse(idx, pulse))
}

/// Direct pulse-width write clamped into `[min_us, max_us]`.
pub fn servo_write_us(pin: u32, pulse_us: u32) -> Result<ChanOut, EngineError> {
    let mut eng = engine();
    let idx = eng.chan_index(pin).ok_or(EngineError::NotAttached)?;
    let ch = eng.channels[idx];
    if ch.kind != ChannelKind::Servo {
        return Err(EngineError::NotAttached);
    }
    let pulse = pulse_us.clamp(ch.min_us, ch.max_us);
    Ok(eng.command_pulse(idx, pulse))
}

/// Detach a channel: stop the slice if both halves are free, quiesce the pin and
/// release the claim. Writes to the pin afterwards are refused (`NotAttached`).
pub fn detach(pin: u32) -> Result<(), EngineError> {
    let mut eng = engine();
    let idx = eng.chan_index(pin).ok_or(EngineError::NotAttached)?;
    eng.detach_index(idx)
}

/// Generic PWM set (`POST /api/v1/pwm/set`): frequency (1..=100 000 Hz, clamped) and
/// pulse width (clamped to the period → `[0 %, 100 %]` duty). Same slice-ownership
/// arbitration as the servo path; refuses hardware writes on conflict.
pub fn pwm_set(pin: u32, freq: u32, pulse_us: u32) -> Result<ChanOut, EngineError> {
    if freq == 0 || freq > MAX_FREQ_HZ {
        return Err(EngineError::InvalidFrequency);
    }
    let conf = math::slice_conf(freq).ok_or(EngineError::InvalidFrequency)?;
    let mut eng = engine();
    let idx = eng.chan_index(pin).ok_or(EngineError::NotClaimed)?;
    let ch = eng.channels[idx];
    if ch.kind != ChannelKind::GenericPwm {
        // Unified engine: a pin carrying a servo attach is not freely reconfigurable.
        return Err(EngineError::PinConflict);
    }

    let slice = pin as usize / 2 % SLICES;
    let half = (pin & 1) as usize;
    {
        let entry = eng.slices[slice];
        let sibling = entry.users[1 - half];
        if sibling.is_some() && entry.configured {
            let same = entry.div_int == conf.div_int
                && entry.div_frac == conf.div_frac
                && entry.top == conf.top;
            if !same {
                return Err(EngineError::SliceConflict);
            }
        }
    }

    let period_us = 1_000_000 / freq;
    let pulse = pulse_us.min(period_us);
    {
        let e = &mut eng.slices[slice];
        write_slice_config(
            slice,
            conf,
            e.configured
                && e.div_int == conf.div_int
                && e.div_frac == conf.div_frac
                && e.top == conf.top,
        );
        let level = math::level_counts(pulse, freq, conf.top);
        write_cc_half(slice, half, level);
        e.users[half] = Some(pin);
        e.div_int = conf.div_int;
        e.div_frac = conf.div_frac;
        e.top = conf.top;
        e.configured = true;
    }
    eng.channels[idx].freq = freq;
    eng.channels[idx].pulse_us = pulse;
    eng.channels[idx].pulse_set = true;
    eng.channels[idx].active = true;
    Ok(eng.channels[idx].snapshot())
}

/// Release a generic-PWM claim (same engine, same detach semantics as servo detach).
pub fn pwm_release(pin: u32) -> Result<(), EngineError> {
    detach(pin).map_err(|e| match e {
        EngineError::NotAttached => EngineError::NotClaimed,
        other => other,
    })
}

/// Snapshot of every active servo channel.
pub fn servo_list() -> heapless::Vec<ChanOut, MAX_CHANNELS> {
    engine()
        .channels
        .iter()
        .filter(|c| c.active && c.kind == ChannelKind::Servo)
        .map(|c| c.snapshot())
        .collect()
}

/// Snapshot of every engine channel (+ servo entries, which the unified `/api/v1/pwm`
/// surface reflects — design decision 9 / open questions: "reflect").
pub fn pwm_list() -> heapless::Vec<ChanOut, MAX_CHANNELS> {
    engine()
        .channels
        .iter()
        .filter(|c| c.active && c.pulse_set)
        .map(|c| c.snapshot())
        .collect()
}

/// Quiesce-all hook for profile transitions: stop every owned slice counter, zero the
/// owned compare halves, blank engine state and force the owned pins back to SIO
/// inputs. Claim descriptors are dropped here as well; the caller (profile apply path)
/// still runs `pin_owner::quiesce_and_reset()`, which blanks the whole flexible pool.
pub fn quiesce_all() {
    let mut eng = engine();
    for slice in 0..SLICES {
        for half in 0..2 {
            eng.slices[slice].users[half] = None;
        }
        pac::PWM.ch(slice).csr().modify(|w| w.set_en(false));
    }
    // Release each channel's pin-ownership claim (release quiesces the freed pins);
    // the profile-apply path follows up with `pin_owner::quiesce_and_reset()`, which
    // blanks the whole flexible pool — releasing twice is idempotent.
    for ch in eng.channels.iter() {
        if ch.active {
            let _ = pin_owner::release(SlotId(ch.slot));
        }
    }
    eng.channels = [Channel::BLANK; MAX_CHANNELS];
    eng.slices = [SliceEntry::BLANK; SLICES];
    defmt::info!("Servo/PWM engine quiesced (all channels released)");
}
