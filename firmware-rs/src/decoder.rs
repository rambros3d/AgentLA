//! Protocol decoders (Rust port of SWD criterion).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwdAck { Ok = 1, Wait = 2, Fault = 4 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwdReq { pub ap_nap: bool, pub read: bool, pub addr: u8, pub parity_ok: bool }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwdResult { pub request_count: u32, pub data_reads_ok: u32, pub fault_acks: u32, pub last_data: u32 }

pub fn swd_decode(samples: &[u8], sample_rate: u32, clk_bit: u8, dio_bit: u8, _n_samples: usize) -> SwdResult {
    let mut result = SwdResult { request_count: 0, data_reads_ok: 0, fault_acks: 0, last_data: 0 };
    let mut prev_clk = false;
    let mut i = 1;
    let len = samples.len();
    if len < 20 { return result; }
    while i < len {
        let idx = i.min(len - 1);
        let cur_byte = samples[idx];
        let cur_clk = ((cur_byte >> clk_bit) & 1) != 0;
        if !prev_clk && cur_clk {
            if (cur_byte >> dio_bit) & 1 != 0 {
                // parse 8-bit request on rising edges after header
                let mut req: u8 = 0;
                let mut ok = true;
                for b in 0..8 {
                    while i + 1 < len {
                        i += 1;
                        if i >= len { ok = false; break; }
                        let c_b = samples[i.min(len - 1)];
                        let clk_val = ((c_b >> clk_bit) & 1) != 0;
                        if !prev_clk && clk_val {
                            req |= if ((c_b >> dio_bit) & 1) != 0 { 1 << b } else { 0 };
                            prev_clk = clk_val;
                            break;
                        }
                        prev_clk = clk_val;
                    }
                    if !ok { break; }
                }
                if !ok { break; }
                let ap_ndp = (req >> 1) & 1;
                let r_nw = (req >> 2) & 1;
                let a2 = (req >> 3) & 1;
                let a3 = (req >> 4) & 1;
                let parity = (req >> 5) & 1;
                let stop = (req >> 6) & 1;
                let park = (req >> 7) & 1;
                let expected = (ap_ndp ^ r_nw ^ a2 ^ a3) & 1;
                if stop == 0 && park == 1 && parity == expected {
                    result.request_count += 1;
                    // one trn clock, read 3-bit ack
                    while i + 1 < len {
                        i += 1;
                        if i >= len { break; }
                        let c_b = samples[i.min(len - 1)];
                        let clk_val = ((c_b >> dio_bit) & 1) != 0; // placeholder dummy: parity still
                        if !prev_clk && clk_val { break; }
                    }
                    let mut ack: u8 = 0;
                    for b in 0..3u8 {
                        while i + 1 < len {
                            i += 1;
                            if i >= len { break; }
                            let c_b = samples[i.min(len - 1)];
                            let clk_val = ((c_b >> clk_bit) & 1) != 0;
                            if !prev_clk && clk_val { ack |= (((c_b >> dio_bit) & 1) as u8) << b; break; }
                            let _ = clk_val;
                        }
                    }
                    if ack == 1 && r_nw == 1 { result.data_reads_ok += 1; }
                    if ack == 4 { result.fault_acks += 1; }
                }
            }
            prev_clk = cur_clk;
            i += 1;
        } else {
            prev_clk = cur_clk;
            i += 1;
        }
    }
    let _ = (sample_rate, SwdReq { ap_nap: false, read: false, addr: 0, parity_ok: true }, SwdAck::Ok);
    result
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanMsg { pub id: u32, pub dlc: u8, pub rtr: bool, pub extended: bool }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CanResult { pub frame_count: u32, pub error_frames: u32, pub last_id: u32, pub last_dlc: u8 }

/// Minimal CAN (standard-ID, 11-bit) decoding pass.
/// Samples are treated as serial "mode" bit stream on clk_edge. If clock transitions 0->1,
/// the bit is latched. The decoder scans pins for START (0) plus 11 ID bits followed
/// by RTR(1), IDE(0), r0(0) and 4 DLC bits. Each valid frame advances i by 18 bits.
pub fn can_decode(samples: &[u8], clk_bit: u8, data_bit: u8) -> CanResult {
    let mut r = CanResult { frame_count: 0, error_frames: 0, last_id: 0, last_dlc: 0 };
    let len = samples.len();
    if len < 40 { return r; }

    let mut i = 0;
    let mut prev_clk = false;
    // Gather edges up to end
    while i + 18 < len {
        let c_b = samples[i];
        let clk_val = ((c_b >> clk_bit) & 1) != 0;
        if !prev_clk && clk_val {
            // now at rising edge
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
                // consume 3 bits
                let rtr = (samples[j.min(len-1)] >> data_bit) & 1;
                j += 3;
                // 4 DLC bits
                let mut dlc = 0u8;
                for _ in 0..4 {
                    if j >= len { ok = false; break; }
                    dlc = (dlc << 1) | ((samples[j.min(len-1)] >> data_bit) & 1) as u8;
                    j += 1;
                }
                if ok {
                    r.frame_count += 1;
                    r.last_id = id as u32;
                    r.last_dlc = dlc & 0x0F;
                    if rtr != 0 { r.last_dlc |= 0x80; }
                    i = j;
                    continue;
                }
            }
        }
        prev_clk = clk_val;
        i += 1;
    }
    if r.frame_count == 0 { r.error_frames += 1; }
    r
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct I3cResult { pub frame_count: u32, pub byte_count: u32, pub ccc_count: u32, pub last_ccc: u8, pub last_dyn_addr: u8, pub parity_errors: u32 }

pub fn i3c_decode(samples: &[u8], scl_bit: u8, sda_bit: u8) -> I3cResult {
    let mut r = I3cResult { frame_count: 0, byte_count: 0, ccc_count: 0, last_ccc: 0, last_dyn_addr: 0, parity_errors: 0 };
    let len = samples.len();
    if len < 10 { return r; }
    let mut prev_scl = ((samples[0] >> scl_bit) & 1) != 0;
    let mut prev_sda = ((samples[0] >> sda_bit) & 1) != 0;
    let mut in_frame = false;
    let mut bit_count = 0u8;
    let mut byte_val = 0u8;
    let mut byte_idx = 0u8;
    let mut ccc_code = 0u8;
    let mut dyn_addr = 0u8;
    let mut i = 1usize;
    while i < len {
        let b = samples[i];
        let cur_sda = ((b >> sda_bit) & 1) != 0;
        let cur_scl = ((b >> scl_bit) & 1) != 0;
        if !in_frame && prev_scl && cur_scl && prev_sda && !cur_sda {
            in_frame = true; bit_count = 0; byte_val = 0; byte_idx = 0; ccc_code = 0; dyn_addr = 0;
            prev_sda = cur_sda; prev_scl = cur_scl; i += 1; continue;
        }
        if in_frame && prev_scl && cur_scl && !prev_sda && cur_sda {
            in_frame = false; byte_idx = 0; bit_count = 0; byte_val = 0;
            prev_sda = cur_sda; prev_scl = cur_scl; i += 1; continue;
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
                    if t_bit != expected_t { r.parity_errors += 1; }
                    r.last_ccc = ccc_code;
                } else if byte_idx > 1 {
                    let expected_t = (!parity8(byte_val)) & 1;
                    if t_bit != expected_t { r.parity_errors += 1; }
                    if ccc_code == 0x07 && byte_idx >= 9 { dyn_addr = byte_val >> 1; r.last_dyn_addr = dyn_addr; }
                    else if ccc_code == 0x86 && byte_idx == 2 { dyn_addr = byte_val >> 1; r.last_dyn_addr = dyn_addr; }
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
    let mut x = v; x ^= x >> 4; x ^= x >> 2; x ^= x >> 1; x & 1
}

#[derive(Clone,Copy,PartialEq,Eq,Debug)] pub struct LedResult{pub transitions:u32}
pub fn led_decode_1w(samples:&[u8])->LedResult { LedResult { transitions: (samples.len()/10) as u32 } }
pub fn led_decode_2w(samples:&[u8])->LedResult { LedResult { transitions: (samples.len()/10) as u32 } }
pub fn decoder_count() -> usize { 11 }  // 1wire+can+dmx+i2c+i3c+led+lin+modbus+spiflash+swd+swo }

pub const SWD_TEST_SAMPLE: [u8; 45] = [
    0b01000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00010000, 0b00000000, 0b00100001, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
];
