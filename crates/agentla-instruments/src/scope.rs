//! Oscilloscope signal analysis and trigger detection
//!
//! Provides waveform metrics (Vpp, Vrms, zero-crossing frequency, duty cycle)
//! and digital edge/threshold trigger detection on ADC sample buffers.

use heapless::Vec;

pub const DEFAULT_VREF_MV: u32 = 3300;
pub const ADC_MAX_VALUE: u16 = 4095;

/// Waveform metrics for a single channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelMetrics {
    pub min_mv: u16,
    pub max_mv: u16,
    pub vpp_mv: u16,
    pub vrms_mv: u16,
    pub freq_hz: u32,
    pub duty_percent: u8,
}

impl Default for ChannelMetrics {
    fn default() -> Self {
        Self {
            min_mv: 0,
            max_mv: 0,
            vpp_mv: 0,
            vrms_mv: 0,
            freq_hz: 0,
            duty_percent: 0,
        }
    }
}

pub type OscilloscopeMetrics = ChannelMetrics;

/// Oscilloscope trigger mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TriggerType {
    #[default]
    None = 0,
    Rising = 1,
    Falling = 2,
    Threshold = 3,
}

/// Digital trigger detector tracking transitions across samples.
#[derive(Debug, Clone, Copy)]
pub struct TriggerDetector {
    pub trigger_type: TriggerType,
    pub threshold: u16,
    last_sample: Option<u16>,
}

impl TriggerDetector {
    pub const fn new(trigger_type: TriggerType, threshold: u16) -> Self {
        Self {
            trigger_type,
            threshold,
            last_sample: None,
        }
    }

    pub fn reset(&mut self) {
        self.last_sample = None;
    }

    /// Process a new sample and return true if a trigger event occurred.
    pub fn process_sample(&mut self, sample: u16) -> bool {
        let sample = sample & ADC_MAX_VALUE;
        let triggered = match self.trigger_type {
            TriggerType::None => true,
            TriggerType::Rising => match self.last_sample {
                Some(prev) => prev < self.threshold && sample >= self.threshold,
                None => false,
            },
            TriggerType::Falling => match self.last_sample {
                Some(prev) => prev > self.threshold && sample <= self.threshold,
                None => false,
            },
            TriggerType::Threshold => sample >= self.threshold,
        };
        self.last_sample = Some(sample);
        triggered
    }

    /// Search a buffer for the first sample index that triggers.
    pub fn find_trigger(&mut self, samples: &[u16]) -> Option<usize> {
        for (i, &sample) in samples.iter().enumerate() {
            if self.process_sample(sample) {
                return Some(i);
            }
        }
        None
    }
}

/// Find the first index in `samples` matching `trigger_type` and `threshold`.
pub fn find_trigger(samples: &[u16], trigger_type: TriggerType, threshold: u16) -> Option<usize> {
    if samples.is_empty() {
        return None;
    }
    match trigger_type {
        TriggerType::None => Some(0),
        TriggerType::Rising => {
            for i in 1..samples.len() {
                if (samples[i - 1] & ADC_MAX_VALUE) < threshold
                    && (samples[i] & ADC_MAX_VALUE) >= threshold
                {
                    return Some(i);
                }
            }
            None
        }
        TriggerType::Falling => {
            for i in 1..samples.len() {
                if (samples[i - 1] & ADC_MAX_VALUE) > threshold
                    && (samples[i] & ADC_MAX_VALUE) <= threshold
                {
                    return Some(i);
                }
            }
            None
        }
        TriggerType::Threshold => {
            for (i, &s) in samples.iter().enumerate() {
                if (s & ADC_MAX_VALUE) >= threshold {
                    return Some(i);
                }
            }
            None
        }
    }
}

/// Integer square root using Newton-Raphson approximation.
pub fn integer_sqrt(val: u64) -> u64 {
    if val == 0 {
        return 0;
    }
    let mut x0 = val / 2;
    if x0 == 0 {
        return 1;
    }
    let mut x1 = (x0 + val / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + val / x0) / 2;
    }
    x0
}

/// Compute metrics for a single channel from a contiguous slice of 12-bit ADC samples.
pub fn compute_metrics(samples: &[u16], vref_mv: u32, sample_rate_hz: u32) -> ChannelMetrics {
    compute_strided_metrics(samples, 1, 0, vref_mv, sample_rate_hz)
}

/// Compute metrics for an interleaved channel with a given `stride` and initial `offset`.
pub fn compute_strided_metrics(
    buffer: &[u16],
    stride: usize,
    offset: usize,
    vref_mv: u32,
    channel_sample_rate_hz: u32,
) -> ChannelMetrics {
    if stride == 0 || offset >= buffer.len() {
        return ChannelMetrics::default();
    }

    let mut min_val = ADC_MAX_VALUE as u32;
    let mut max_val = 0u32;
    let mut sum_sq: u64 = 0;
    let mut count: u32 = 0;

    let mut idx = offset;
    while idx < buffer.len() {
        let val = (buffer[idx] & ADC_MAX_VALUE) as u32;
        if val < min_val {
            min_val = val;
        }
        if val > max_val {
            max_val = val;
        }
        let mv = (val * vref_mv) / (ADC_MAX_VALUE as u32);
        sum_sq += (mv as u64) * (mv as u64);
        count += 1;
        idx += stride;
    }

    if count == 0 {
        return ChannelMetrics::default();
    }

    let min_mv = ((min_val * vref_mv) / (ADC_MAX_VALUE as u32)) as u16;
    let max_mv = ((max_val * vref_mv) / (ADC_MAX_VALUE as u32)) as u16;
    let vpp_mv = max_mv.saturating_sub(min_mv);

    // Vrms = sqrt(sum(V^2) / N)
    let mean_sq = sum_sq / (count as u64);
    let vrms_mv = integer_sqrt(mean_sq) as u16;

    // Frequency and duty estimation via midpoint crossings
    let mid_val = (min_val + max_val) / 2;
    let mut high_count = 0u32;
    let mut crossings = 0u32;
    let mut last_state = ((buffer[offset] & ADC_MAX_VALUE) as u32) > mid_val;

    let mut first_rising_sample: Option<u32> = None;
    let mut last_rising_sample: Option<u32> = None;
    let mut rising_count = 0u32;

    let mut first_crossing_sample: Option<u32> = None;
    let mut last_crossing_sample: Option<u32> = None;

    let mut sample_pos = 0u32;
    let mut sample_idx = offset;
    while sample_idx < buffer.len() {
        let val = (buffer[sample_idx] & ADC_MAX_VALUE) as u32;
        let state = val > mid_val;
        if state {
            high_count += 1;
        }
        if state != last_state {
            crossings += 1;
            if first_crossing_sample.is_none() {
                first_crossing_sample = Some(sample_pos);
            }
            last_crossing_sample = Some(sample_pos);

            if state {
                // Rising edge crossing
                rising_count += 1;
                if first_rising_sample.is_none() {
                    first_rising_sample = Some(sample_pos);
                }
                last_rising_sample = Some(sample_pos);
            }
            last_state = state;
        }
        sample_idx += stride;
        sample_pos += 1;
    }

    let duty_percent = (((high_count as u64) * 100) / (count as u64)) as u8;

    // Frequency estimation
    let freq_hz = if vpp_mv > 100 {
        if rising_count >= 2 {
            let cycles = (rising_count - 1) as u64;
            let sample_span = (last_rising_sample.unwrap() - first_rising_sample.unwrap()) as u64;
            if sample_span > 0 {
                ((cycles * channel_sample_rate_hz as u64) / sample_span) as u32
            } else {
                0
            }
        } else if crossings >= 2 {
            let half_cycles = (crossings - 1) as u64;
            let sample_span = (last_crossing_sample.unwrap() - first_crossing_sample.unwrap()) as u64;
            if sample_span > 0 {
                ((half_cycles * channel_sample_rate_hz as u64) / (2 * sample_span)) as u32
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };

    ChannelMetrics {
        min_mv,
        max_mv,
        vpp_mv,
        vrms_mv,
        freq_hz,
        duty_percent,
    }
}

/// Compute metrics across 4 channels from an interleaved ADC sample buffer.
pub fn compute_multi_channel_metrics(
    buffer: &[u16],
    ch_mask: u8,
    sample_rate: u32,
    vref_mv: u32,
    metrics: &mut [ChannelMetrics; 4],
) {
    let mut active_chans: Vec<usize, 4> = Vec::new();
    for i in 0..4 {
        if (ch_mask & (1 << i)) != 0 {
            let _ = active_chans.push(i);
        }
    }
    if active_chans.is_empty() {
        let _ = active_chans.push(0);
    }

    let num_ch = active_chans.len();
    let ch_sample_rate = sample_rate / (num_ch as u32).max(1);

    for (pos, &ch_idx) in active_chans.iter().enumerate() {
        metrics[ch_idx] = compute_strided_metrics(
            buffer,
            num_ch,
            pos,
            vref_mv,
            ch_sample_rate,
        );
    }
}
