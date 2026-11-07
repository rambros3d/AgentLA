//! MIPI I3C protocol decoder.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct I3cResult {
    pub frame_count: u32,
    pub byte_count: u32,
    pub ccc_count: u32,
    pub last_ccc: u8,
    pub last_dyn_addr: u8,
    pub parity_errors: u32,
}

pub fn i3c_decode(samples: &[u8], scl_bit: u8, sda_bit: u8) -> I3cResult {
    let mut r = I3cResult {
        frame_count: 0,
        byte_count: 0,
        ccc_count: 0,
        last_ccc: 0,
        last_dyn_addr: 0,
        parity_errors: 0,
    };
    let len = samples.len();
    if len < 10 {
        return r;
    }
    let mut prev_scl = ((samples[0] >> scl_bit) & 1) != 0;
    let mut prev_sda = ((samples[0] >> sda_bit) & 1) != 0;
    let mut in_frame = false;
    let mut bit_count = 0u8;
    let mut byte_val = 0u8;
    let mut byte_idx = 0u8;
    let mut ccc_code = 0u8;
    let mut i = 1usize;
    while i < len {
        let b = samples[i];
        let cur_sda = ((b >> sda_bit) & 1) != 0;
        let cur_scl = ((b >> scl_bit) & 1) != 0;
        if !in_frame && prev_scl && cur_scl && prev_sda && !cur_sda {
            in_frame = true;
            bit_count = 0;
            byte_val = 0;
            byte_idx = 0;
            ccc_code = 0;
            prev_sda = cur_sda;
            prev_scl = cur_scl;
            i += 1;
            continue;
        }
        if in_frame && prev_scl && cur_scl && !prev_sda && cur_sda {
            in_frame = false;
            byte_idx = 0;
            bit_count = 0;
            byte_val = 0;
            prev_sda = cur_sda;
            prev_scl = cur_scl;
            i += 1;
            continue;
        }
        if in_frame && !prev_scl && cur_scl {
            if bit_count < 8 {
                byte_val = (byte_val << 1) | cur_sda as u8;
                bit_count += 1;
            } else {
                let t_bit = cur_sda as u8;
                if byte_idx == 1 {
                    ccc_code = byte_val;
                    r.ccc_count += 1;
                    let expected_t = (!parity8(byte_val)) & 1;
                    if t_bit != expected_t {
                        r.parity_errors += 1;
                    }
                    r.last_ccc = ccc_code;
                } else if byte_idx > 1 {
                    let expected_t = (!parity8(byte_val)) & 1;
                    if t_bit != expected_t {
                        r.parity_errors += 1;
                    }
                    if ccc_code == 0x07 && byte_idx >= 9 {
                        r.last_dyn_addr = byte_val >> 1;
                    } else if ccc_code == 0x86 && byte_idx == 2 {
                        r.last_dyn_addr = byte_val >> 1;
                    }
                }
                r.byte_count = r.byte_count.saturating_add(1);
                byte_idx += 1;
                bit_count = 0;
                byte_val = 0;
            }
        }
        prev_sda = cur_sda;
        prev_scl = cur_scl;
        i += 1;
    }
    r.frame_count = if r.ccc_count > 0 { 1 } else { 0 };
    r
}

fn parity8(v: u8) -> u8 {
    let mut x = v;
    x ^= x >> 4;
    x ^= x >> 2;
    x ^= x >> 1;
    x & 1
}
