//! CAN bus decoder.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanMsg {
    pub id: u32,
    pub dlc: u8,
    pub rtr: bool,
    pub extended: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanResult {
    pub frame_count: u32,
    pub error_frames: u32,
    pub last_id: u32,
    pub last_dlc: u8,
}

/// Minimal CAN (standard-ID, 11-bit) decoding pass.
/// Samples are treated as serial bit stream on clk_edge. If clock transitions 0->1,
/// the bit is latched. The decoder scans pins for START (0) plus 11 ID bits followed
/// by RTR(1), IDE(0), r0(0) and 4 DLC bits. Each valid frame advances i by 18 bits.
pub fn can_decode(samples: &[u8], clk_bit: u8, data_bit: u8) -> CanResult {
    let mut r = CanResult {
        frame_count: 0,
        error_frames: 0,
        last_id: 0,
        last_dlc: 0,
    };
    let len = samples.len();
    if len < 40 {
        return r;
    }

    let mut i = 0;
    let mut prev_clk = false;
    while i + 18 < len {
        let c_b = samples[i];
        let clk_val = ((c_b >> clk_bit) & 1) != 0;
        if !prev_clk && clk_val {
            if (c_b >> data_bit) & 1 != 0 {
                // Not a SOF (sof is dominant 0); skip
                prev_clk = clk_val;
                i += 1;
                continue;
            }
            // consume 11 ID bits
            let mut j = i + 1;
            let mut id: u16 = 0;
            let mut bits = 0;
            let mut ok = true;
            while bits < 11 && j < len {
                let b = samples[j.min(len - 1)];
                let cval = ((b >> clk_bit) & 1) != 0;
                if !prev_clk && cval {
                    id = (id << 1) | (((b >> data_bit) & 1) as u16);
                    bits += 1;
                    prev_clk = cval;
                } else {
                    prev_clk = cval;
                }
                j += 1;
            }
            if bits == 11 && j < len {
                // expect RTR=0 (data), IDE=0 (std), R0=0
                let rtr = (samples[j.min(len - 1)] >> data_bit) & 1;
                j += 3;
                let mut dlc = 0u8;
                for _ in 0..4 {
                    if j >= len {
                        ok = false;
                        break;
                    }
                    dlc = (dlc << 1) | ((samples[j.min(len - 1)] >> data_bit) & 1) as u8;
                    j += 1;
                }
                if ok {
                    r.frame_count += 1;
                    r.last_id = id as u32;
                    r.last_dlc = dlc & 0x0F;
                    if rtr != 0 {
                        r.last_dlc |= 0x80;
                    }
                    i = j;
                    continue;
                }
            }
        }
        prev_clk = clk_val;
        i += 1;
    }
    if r.frame_count == 0 {
        r.error_frames += 1;
    }
    r
}
