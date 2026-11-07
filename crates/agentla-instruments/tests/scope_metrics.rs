use agentla_instruments::scope::{
    compute_metrics, compute_multi_channel_metrics, find_trigger, integer_sqrt, ChannelMetrics,
    TriggerDetector, TriggerType,
};

#[test]
fn test_integer_sqrt() {
    assert_eq!(integer_sqrt(0), 0);
    assert_eq!(integer_sqrt(1), 1);
    assert_eq!(integer_sqrt(2), 1);
    assert_eq!(integer_sqrt(3), 1);
    assert_eq!(integer_sqrt(4), 2);
    assert_eq!(integer_sqrt(9), 3);
    assert_eq!(integer_sqrt(15), 3);
    assert_eq!(integer_sqrt(16), 4);
    assert_eq!(integer_sqrt(100), 10);
    assert_eq!(integer_sqrt(1_000_000), 1000);
    assert_eq!(integer_sqrt(1_000_000_000_000), 1_000_000);
}

#[test]
fn test_synthetic_sine_wave_metrics() {
    // 1 kHz sine wave, sampled at 100 kS/s for 1000 samples (10 full cycles)
    // Centered at 1650 mV with amplitude 1000 mV (Vmin = 650 mV, Vmax = 2650 mV, Vpp = 2000 mV)
    // Vref = 3300 mV
    const SAMPLE_RATE: u32 = 100_000;
    const NUM_SAMPLES: usize = 1000;
    const FREQ: f64 = 1000.0;
    const VREF_MV: u32 = 3300;

    let mut samples = [0u16; NUM_SAMPLES];
    for i in 0..NUM_SAMPLES {
        let t = i as f64 / SAMPLE_RATE as f64;
        let v_mv = 1650.0 + 1000.0 * (2.0 * core::f64::consts::PI * FREQ * t).sin();
        let raw_adc = ((v_mv * 4095.0) / VREF_MV as f64).round() as u16;
        samples[i] = raw_adc;
    }

    let metrics = compute_metrics(&samples, VREF_MV, SAMPLE_RATE);

    // Expected Vpp: 2000 mV, within 2% margin
    let vpp_expected = 2000.0;
    let vpp_err = (metrics.vpp_mv as f64 - vpp_expected).abs() / vpp_expected;
    assert!(
        vpp_err < 0.02,
        "Vpp error {}% exceeds 2% margin: actual={}, expected={}",
        vpp_err * 100.0,
        metrics.vpp_mv,
        vpp_expected
    );

    // Expected Vrms: sqrt(1650^2 + 1000^2 / 2) = sqrt(3,222,500) ≈ 1795 mV
    let vrms_expected = (1650.0_f64.powi(2) + 1000.0_f64.powi(2) / 2.0).sqrt();
    let vrms_err = (metrics.vrms_mv as f64 - vrms_expected).abs() / vrms_expected;
    assert!(
        vrms_err < 0.02,
        "Vrms error {}% exceeds 2% margin: actual={}, expected={}",
        vrms_err * 100.0,
        metrics.vrms_mv,
        vrms_expected
    );

    // Expected Frequency: 1000 Hz within 2% margin
    let freq_err = (metrics.freq_hz as f64 - FREQ).abs() / FREQ;
    assert!(
        freq_err < 0.02,
        "Frequency error {}% exceeds 2% margin: actual={}, expected={}",
        freq_err * 100.0,
        metrics.freq_hz,
        FREQ
    );

    // Duty cycle should be ~50%
    assert!(
        metrics.duty_percent >= 48 && metrics.duty_percent <= 52,
        "Duty percent {} not close to 50%",
        metrics.duty_percent
    );
}

#[test]
fn test_synthetic_square_wave() {
    // 2 kHz square wave, 50% duty, sampled at 100 kS/s for 1000 samples
    const SAMPLE_RATE: u32 = 100_000;
    const NUM_SAMPLES: usize = 1000;
    const VREF_MV: u32 = 3300;

    let mut samples = [0u16; NUM_SAMPLES];
    for i in 0..NUM_SAMPLES {
        let period_samples = SAMPLE_RATE / 2000;
        let is_high = (i as u32 % period_samples) < (period_samples / 2);
        // Low: 300 mV (ADC ~372), High: 3000 mV (ADC ~3722)
        samples[i] = if is_high {
            ((3000 * 4095) / VREF_MV) as u16
        } else {
            ((300 * 4095) / VREF_MV) as u16
        };
    }

    let metrics = compute_metrics(&samples, VREF_MV, SAMPLE_RATE);
    assert_eq!(metrics.freq_hz, 2000);
    assert_eq!(metrics.duty_percent, 50);
    assert!((metrics.vpp_mv as i32 - 2700).abs() <= 5);
}

#[test]
fn test_dc_signal() {
    const SAMPLE_RATE: u32 = 100_000;
    const VREF_MV: u32 = 3300;
    let samples = [2048u16; 500]; // ~1650 mV DC

    let metrics = compute_metrics(&samples, VREF_MV, SAMPLE_RATE);
    assert_eq!(metrics.vpp_mv, 0);
    assert_eq!(metrics.freq_hz, 0);
    assert!((metrics.vrms_mv as i32 - 1650).abs() <= 5);
}

#[test]
fn test_multi_channel_interleaved_metrics() {
    const TOTAL_SAMPLES: usize = 1024;
    let mut buffer = [0u16; TOTAL_SAMPLES];

    // Interleave 4 channels:
    // Ch 0: 1000 Hz square wave
    // Ch 1: 500 Hz square wave
    // Ch 2: Flat DC 1.0 V
    // Ch 3: Flat DC 2.5 V
    for i in 0..TOTAL_SAMPLES {
        let ch = i % 4;
        let sample_num = i / 4;
        match ch {
            0 => {
                // 1000 Hz with ch_sample_rate = 25000 -> period = 25 samples
                buffer[i] = if (sample_num % 25) < 12 { 3722 } else { 372 };
            }
            1 => {
                // 500 Hz with ch_sample_rate = 25000 -> period = 50 samples
                buffer[i] = if (sample_num % 50) < 25 { 3722 } else { 372 };
            }
            2 => {
                buffer[i] = 1241; // ~1000 mV
            }
            3 => {
                buffer[i] = 3102; // ~2500 mV
            }
            _ => unreachable!(),
        }
    }

    let mut metrics = [ChannelMetrics::default(); 4];
    compute_multi_channel_metrics(&buffer, 0x0F, 100_000, 3300, &mut metrics);

    // Ch 0 frequency: ~1000 Hz
    let err0 = (metrics[0].freq_hz as i32 - 1000).abs();
    assert!(err0 <= 20, "Ch 0 freq {} not ~1000 Hz", metrics[0].freq_hz);

    // Ch 1 frequency: ~500 Hz
    let err1 = (metrics[1].freq_hz as i32 - 500).abs();
    assert!(err1 <= 20, "Ch 1 freq {} not ~500 Hz", metrics[1].freq_hz);

    // Ch 2 DC
    assert_eq!(metrics[2].freq_hz, 0);
    assert_eq!(metrics[2].vpp_mv, 0);

    // Ch 3 DC
    assert_eq!(metrics[3].freq_hz, 0);
    assert_eq!(metrics[3].vpp_mv, 0);
}

#[test]
fn test_trigger_detector() {
    let samples = [100u16, 200, 300, 1500, 2500, 2200, 1800, 800, 200];

    // Trigger None
    assert_eq!(find_trigger(&samples, TriggerType::None, 2000), Some(0));

    // Rising edge past 2000: index 4 (from 1500 to 2500)
    assert_eq!(find_trigger(&samples, TriggerType::Rising, 2000), Some(4));

    // Falling edge below 1000: index 7 (from 1800 to 800)
    assert_eq!(find_trigger(&samples, TriggerType::Falling, 1000), Some(7));

    // Threshold >= 2000: index 4
    assert_eq!(find_trigger(&samples, TriggerType::Threshold, 2000), Some(4));

    // No match
    assert_eq!(find_trigger(&samples, TriggerType::Rising, 3500), None);

    // Stateful detector
    let mut det = TriggerDetector::new(TriggerType::Rising, 1000);
    assert!(!det.process_sample(500));
    assert!(!det.process_sample(800));
    assert!(det.process_sample(1200)); // Triggered!
    assert!(!det.process_sample(1500));
}
