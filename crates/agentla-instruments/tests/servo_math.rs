use agentla_instruments::servo::{
    angle_to_pulse, clamp_pulse_us, continuous_pulse, level_counts, slice_conf, SliceConf,
    SERVO_50HZ, SYS_CLK_HZ,
};

const fn freq_actual_hz(conf: SliceConf) -> u64 {
    let denom = (conf.top as u64 + 1) * (16u64 * conf.div_int as u64 + conf.div_frac as u64);
    (SYS_CLK_HZ as u64 * 16) / denom
}

fn assert_freq(freq: u32, conf: SliceConf) {
    let actual = freq_actual_hz(conf) as u32;
    let err = actual.abs_diff(freq);
    assert!(
        err as u64 * 1000 <= freq as u64 * 10, // <= 1%
        "freq {} -> {:?} -> actual {} Hz",
        freq,
        conf,
        actual
    );
}

#[test]
fn test_50hz_servo_parameters() {
    // Scenario: Calculating 50 Hz servo PWM parameters
    // WHEN calculating timing for 50 Hz with 150 MHz system clock
    // THEN the calculator yields integer divisor 150, fractional divisor 0, and top count 19,999 (1 µs resolution).
    let conf = slice_conf(50);
    assert_eq!(
        conf,
        Some(SliceConf {
            div_int: 150,
            div_frac: 0,
            top: 19999,
        })
    );
    assert_eq!(conf.unwrap(), SERVO_50HZ);
    assert_eq!(freq_actual_hz(conf.unwrap()), 50);
}

#[test]
fn test_generic_paths_hit_frequency() {
    // RP2350 PWM with 150 MHz sys clock reaches down to ~8.97 Hz.
    for freq in [10, 100, 1000, 10000, 100000, 333, 55555] {
        let conf = slice_conf(freq).unwrap();
        assert!((1..=255).contains(&conf.div_int), "div_int {:?}", conf);
        assert!(conf.div_frac <= 15, "div_frac {:?}", conf);
        assert!((1..=65534).contains(&conf.top), "top {:?}", conf);
        assert_freq(freq, conf);
    }

    // Known exact case: 1 kHz
    assert_eq!(
        slice_conf(1000),
        Some(SliceConf {
            div_int: 2,
            div_frac: 5,
            top: 64863,
        })
    );

    // 100 kHz
    assert_eq!(
        slice_conf(100000),
        Some(SliceConf {
            div_int: 1,
            div_frac: 0,
            top: 1499,
        })
    );

    // Degenerate low frequency (< 9 Hz wrap clamped, error accepted per spec)
    let conf = slice_conf(1).unwrap();
    assert_eq!(conf.div_int, 255);
    assert_eq!(conf.div_frac, 0);
    assert_eq!(conf.top, 65534);
}

#[test]
fn test_invalid_frequency_rejected() {
    assert_eq!(slice_conf(0), None);
}

#[test]
fn test_level_math_proportional() {
    // Half period at 1 kHz -> half of (top + 1)
    let conf = slice_conf(1000).unwrap();
    assert_eq!(level_counts(500, 1000, conf.top), (conf.top + 1) / 2);
    assert_eq!(level_counts(0, 1000, conf.top), 0);
    assert_eq!(level_counts(1000, 1000, conf.top), conf.top + 1);
    assert_eq!(level_counts(u32::MAX, 1000, conf.top), conf.top + 1); // clamped

    // 50 Hz fast path: 1 µs per count
    assert_eq!(level_counts(1500, 50, 19999), 1500);
}

#[test]
fn test_continuous_throttle_mapping() {
    // Deadband |t| <= 5 -> neutral 1500 µs
    for t in [-5, 0, 5] {
        assert_eq!(continuous_pulse(t, 500, 2500), 1500);
    }
    // Full reverse/forward reach the limits
    assert_eq!(continuous_pulse(-100, 500, 2500), 500);
    assert_eq!(continuous_pulse(100, 500, 2500), 2500);
    // Just past the deadband: forward 1520+σ, reverse 1480-σ
    assert_eq!(continuous_pulse(6, 500, 2500), 1520 + (1000 / 95));
    assert_eq!(continuous_pulse(-6, 500, 2500), 1480 - (980 / 95));
    // Midpoint sanity
    assert!(continuous_pulse(50, 500, 2500) > 1500);
    assert!(continuous_pulse(-50, 500, 2500) < 1500);
}

#[test]
fn test_angle_and_clamp() {
    assert_eq!(angle_to_pulse(0, 500, 2500), 500);
    assert_eq!(angle_to_pulse(90, 500, 2500), 1500);
    assert_eq!(angle_to_pulse(180, 500, 2500), 2500);
    assert_eq!(angle_to_pulse(200, 500, 2500), 2500); // clamped to 180

    assert_eq!(clamp_pulse_us(300, 500, 2500), 500);
    assert_eq!(clamp_pulse_us(1500, 500, 2500), 1500);
    assert_eq!(clamp_pulse_us(3000, 500, 2500), 2500);
}
