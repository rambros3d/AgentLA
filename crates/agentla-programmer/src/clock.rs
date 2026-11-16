//! PIO clock divider calculation and protocol clock mapping utilities.

/// Computes 16-bit integer and 8-bit fractional PIO clock dividers for a target frequency.
///
/// Guarantees that the synthesized clock frequency does not exceed `target_freq_hz`.
/// PIO clock formula:
/// `f_bit = (sys_clk_hz / (int_div + frac_div / 256)) / cycles_per_bit`
pub fn calculate_pio_clkdiv(sys_clk_hz: u32, target_freq_hz: u32, cycles_per_bit: u32) -> (u16, u8) {
    if target_freq_hz == 0 || cycles_per_bit == 0 {
        return (65535, 255);
    }
    let denom = (target_freq_hz as u64) * (cycles_per_bit as u64);
    let total_substeps = (sys_clk_hz as u64 * 256 + denom - 1) / denom;
    if total_substeps < 256 {
        (1, 0)
    } else if total_substeps >= (65535u64 << 8) + 255 {
        (65535, 255)
    } else {
        let int_div = (total_substeps >> 8) as u16;
        let frac_div = (total_substeps & 0xFF) as u8;
        (int_div, frac_div)
    }
}

/// Maps ST-Link frequency divisor index values to safe target frequencies in kHz.
///
/// Divisor 0 maps to 4000 kHz (4.0 MHz).
/// Divisor N > 0 maps to floor(3600 / (N + 1)) kHz.
pub fn swd_divisor_to_khz(div: u16) -> u32 {
    if div == 0 {
        4000
    } else {
        3600 / (div as u32 + 1)
    }
}

/// Maps target SWD frequency in kHz back to ST-Link divisor index.
///
/// Chooses a divisor such that the resulting frequency does not exceed `freq_khz`.
pub fn swd_khz_to_divisor(freq_khz: u32) -> u16 {
    if freq_khz >= 4000 {
        0
    } else if freq_khz == 0 {
        u16::MAX
    } else {
        let div_plus_1 = (3600 + freq_khz - 1) / freq_khz;
        let div = div_plus_1.saturating_sub(1);
        div.min(u16::MAX as u32) as u16
    }
}

/// Maps WCH-Link speed tier codes to target frequencies in kHz.
///
/// - Tier 1: 6000 kHz (High speed, 6.0 MHz)
/// - Tier 2: 4000 kHz (Medium speed, 4.0 MHz)
/// - Tier 3: 400 kHz (Low speed)
/// - Other: Defaults to 6000 kHz (High speed)
pub fn wch_speed_to_khz(speed_tier: u8) -> u32 {
    match speed_tier {
        1 => 6000,
        2 => 4000,
        3 => 400,
        _ => 6000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_spec_scenarios() {
        // Spec: Synthesize 4 MHz clock from 150 MHz system clock with 2 cycles per bit
        let (int_div, frac_div) = calculate_pio_clkdiv(150_000_000, 4_000_000, 2);
        assert_eq!(int_div, 18);
        assert_eq!(frac_div, 192);

        // Spec: Synthesize 1 MHz clock from 150 MHz system clock with 2 cycles per bit
        let (int_div, frac_div) = calculate_pio_clkdiv(150_000_000, 1_000_000, 2);
        assert_eq!(int_div, 75);
        assert_eq!(frac_div, 0);

        // Spec: ST-Link divisor index 0 maps to 4000 kHz
        assert_eq!(swd_divisor_to_khz(0), 4000);

        // Spec: ST-Link divisor index 1 maps to 1800 kHz
        assert_eq!(swd_divisor_to_khz(1), 1800);

        // Spec: WCH speed tier 1 maps to 6000 kHz
        assert_eq!(wch_speed_to_khz(1), 6000);

        // Spec: WCH speed tier 2 maps to 4000 kHz
        assert_eq!(wch_speed_to_khz(2), 4000);

        // Spec: WCH speed tier 3 maps to 400 kHz
        assert_eq!(wch_speed_to_khz(3), 400);

        // Default tier maps to 6000 kHz
        assert_eq!(wch_speed_to_khz(0), 6000);
        assert_eq!(wch_speed_to_khz(4), 6000);
        assert_eq!(wch_speed_to_khz(255), 6000);
    }

    #[test]
    fn test_stlink_divisor_mapping() {
        assert_eq!(swd_divisor_to_khz(2), 1200);
        assert_eq!(swd_divisor_to_khz(3), 900);
        assert_eq!(swd_divisor_to_khz(798), 4);

        assert_eq!(swd_khz_to_divisor(4000), 0);
        assert_eq!(swd_khz_to_divisor(5000), 0);
        assert_eq!(swd_khz_to_divisor(1800), 1);
        assert_eq!(swd_khz_to_divisor(1200), 2);
        assert_eq!(swd_khz_to_divisor(900), 3);
        assert_eq!(swd_khz_to_divisor(0), u16::MAX);

        // Ensure swd_divisor_to_khz(swd_khz_to_divisor(f)) <= f
        for freq in [5, 10, 50, 100, 400, 900, 1000, 1200, 1800, 2000, 4000] {
            let div = swd_khz_to_divisor(freq);
            let mapped_khz = swd_divisor_to_khz(div);
            assert!(mapped_khz <= freq, "freq {} mapped to higher {}", freq, mapped_khz);
        }
    }

    #[test]
    fn test_divider_math_across_ranges() {
        let sys_clk = 150_000_000u32;
        // Test frequencies from 5 kHz to 10 MHz
        let test_frequencies = [
            5_000,
            10_000,
            50_000,
            100_000,
            250_000,
            400_000,
            1_000_000,
            2_000_000,
            4_000_000,
            6_000_000,
            8_000_000,
            10_000_000,
        ];

        for &cycles in &[2u32, 4u32, 7u32] {
            for &target_freq in &test_frequencies {
                let (int_div, frac_div) = calculate_pio_clkdiv(sys_clk, target_freq, cycles);
                assert!(int_div >= 1, "int_div must be >= 1");
                let divider_f64 = (int_div as f64) + (frac_div as f64) / 256.0;
                let synth_freq = (sys_clk as f64) / (divider_f64 * cycles as f64);

                // Guarantee synthesized clock frequency does not exceed target
                assert!(
                    synth_freq <= (target_freq as f64) + 1e-6,
                    "target: {}, synth: {}, int: {}, frac: {}, cycles: {}",
                    target_freq, synth_freq, int_div, frac_div, cycles
                );

                // Guarantee precision within 1% for standard ranges
                let error_ratio = ((target_freq as f64) - synth_freq) / (target_freq as f64);
                assert!(
                    error_ratio < 0.01,
                    "error ratio too high ({}) for target {} synth {}",
                    error_ratio, target_freq, synth_freq
                );
            }
        }
    }

    #[test]
    fn test_divider_boundary_conditions() {
        assert_eq!(calculate_pio_clkdiv(150_000_000, 0, 4), (65535, 255));
        assert_eq!(calculate_pio_clkdiv(150_000_000, 1_000_000, 0), (65535, 255));
        // Target exceeding system clock
        assert_eq!(calculate_pio_clkdiv(150_000_000, 200_000_000, 1), (1, 0));
    }
}
