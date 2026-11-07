//! Value Change Dump (VCD) text generator.

use core::fmt::Write as _;

pub const CH_SYMBOLS: [char; 8] = ['!', '"', '#', '$', '%', '&', '\'', '('];

/// Formats standard VCD output from a sample word slice into the target heapless string.
pub fn generate_vcd(samples: &[u32], out: &mut heapless::String<2048>) {
    generate_vcd_custom(samples, 8, 128, out);
}

/// Formats VCD output with custom channel count (up to 8) and sample limit.
pub fn generate_vcd_custom(
    samples: &[u32],
    channel_count: usize,
    max_samples: usize,
    out: &mut heapless::String<2048>,
) {
    let _ = write!(
        out,
        "$date Today $end\n$version AgentLA v1.0-rust $end\n$timescale 10ns $end\n$scope module top $end\n"
    );
    let channels = channel_count.min(8);
    for ch in 0..channels {
        let _ = write!(out, "$var wire 1 {} ch{} $end\n", CH_SYMBOLS[ch], ch);
    }
    let _ = write!(out, "$upscope $end\n$enddefinitions $end\n#0\n");

    let limit = samples.len().min(max_samples);
    let mut prev_word = 0xFFFFFFFFu32;
    for t in 0..limit {
        let w = samples[t];
        if w != prev_word {
            let _ = write!(out, "#{}\n", t * 10);
            for ch in 0..channels {
                let bit = (w >> ch) & 1;
                let prev_bit = (prev_word >> ch) & 1;
                if bit != prev_bit || t == 0 {
                    let _ = write!(out, "{}{}\n", bit, CH_SYMBOLS[ch]);
                }
            }
            prev_word = w;
        }
    }
}
