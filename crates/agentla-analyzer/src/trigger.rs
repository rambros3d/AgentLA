//! Trigger qualification logic.

/// Counts rising-edge transitions on `trig_bit` across a sequence of 8-bit sample bytes.
pub fn rising_edge(samples: &[u8], trig_bit: u8) -> usize {
    let mut count = 0usize;
    let n = samples.len();
    if n == 0 {
        return 0;
    }
    let mut prev = ((samples[0] >> trig_bit) & 1) != 0;
    let mut i = 1usize;
    while i < n {
        let cur = ((samples[i] >> trig_bit) & 1) != 0;
        if !prev && cur {
            count += 1;
        }
        prev = cur;
        i += 1;
    }
    count
}

/// Counts falling-edge transitions on `trig_bit` across a sequence of 8-bit sample bytes.
pub fn falling_edge(samples: &[u8], trig_bit: u8) -> usize {
    let mut count = 0usize;
    let n = samples.len();
    if n == 0 {
        return 0;
    }
    let mut prev = ((samples[0] >> trig_bit) & 1) != 0;
    let mut i = 1usize;
    while i < n {
        let cur = ((samples[i] >> trig_bit) & 1) != 0;
        if prev && !cur {
            count += 1;
        }
        prev = cur;
        i += 1;
    }
    count
}
