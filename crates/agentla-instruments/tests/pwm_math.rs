use agentla_instruments::generator::{
    calculate_slice_params, duty_to_level, PwmTiming, DEFAULT_SYS_CLK_HZ,
};

#[test]
fn test_high_frequency_square_wave_10mhz() {
    // Scenario: Calculating high-frequency square wave parameters
    // WHEN calculating timing for 10 MHz with 150 MHz system clock
    // THEN the calculator yields integer divisor 1, fractional divisor 0, and top count 14 (50% duty at count 7).
    let timing = calculate_slice_params(DEFAULT_SYS_CLK_HZ, 10_000_000);
    assert_eq!(
        timing,
        PwmTiming {
            div_int: 1,
            div_frac: 0,
            top: 14,
        }
    );

    let level_50 = duty_to_level(timing.top, 50);
    assert_eq!(level_50, 7);
}

#[test]
fn test_extreme_high_frequencies() {
    // 75 MHz
    let t75 = calculate_slice_params(DEFAULT_SYS_CLK_HZ, 75_000_000);
    assert_eq!(t75, PwmTiming { div_int: 1, div_frac: 0, top: 1 });

    // > 75 MHz clamped
    let t100 = calculate_slice_params(DEFAULT_SYS_CLK_HZ, 100_000_000);
    assert_eq!(t100, PwmTiming { div_int: 1, div_frac: 0, top: 1 });
}

#[test]
fn test_duty_to_level() {
    let top = 19999u16;
    assert_eq!(duty_to_level(top, 0), 0);
    assert_eq!(duty_to_level(top, 100), 20000);
    assert_eq!(duty_to_level(top, 50), 10000);
    assert_eq!(duty_to_level(top, 25), 5000);
    assert_eq!(duty_to_level(top, 75), 15000);
}

#[test]
fn test_generator_generic_frequencies() {
    for freq in [100, 1_000, 10_000, 100_000, 1_000_000] {
        let timing = calculate_slice_params(DEFAULT_SYS_CLK_HZ, freq);
        assert!((1..=255).contains(&timing.div_int));
        assert!(timing.div_frac <= 15);
        assert!(timing.top > 0);

        // Verify effective frequency is close to target
        let divisor_x16 = timing.div_int as u64 * 16 + timing.div_frac as u64;
        let denom = (timing.top as u64 + 1) * divisor_x16;
        let actual_freq = (DEFAULT_SYS_CLK_HZ as u64 * 16) / denom;
        let err = actual_freq.abs_diff(freq as u64);
        assert!(
            err * 100 <= freq as u64 * 2, // Within 2%
            "freq {} -> timing {:?} -> actual {} Hz",
            freq,
            timing,
            actual_freq
        );
    }
}
