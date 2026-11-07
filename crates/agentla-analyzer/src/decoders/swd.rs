//! Arm SWD signal decoder.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwdAck {
    Ok = 1,
    Wait = 2,
    Fault = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwdReq {
    pub ap_nap: bool,
    pub read: bool,
    pub addr: u8,
    pub parity_ok: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SwdResult {
    pub request_count: u32,
    pub data_reads_ok: u32,
    pub fault_acks: u32,
    pub last_data: u32,
}

pub fn swd_decode(
    samples: &[u8],
    sample_rate: u32,
    clk_bit: u8,
    dio_bit: u8,
    _n_samples: usize,
) -> SwdResult {
    let mut result = SwdResult {
        request_count: 0,
        data_reads_ok: 0,
        fault_acks: 0,
        last_data: 0,
    };
    let mut prev_clk = false;
    let mut i = 1;
    let len = samples.len();
    if len < 20 {
        return result;
    }
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
                        if i >= len {
                            ok = false;
                            break;
                        }
                        let c_b = samples[i.min(len - 1)];
                        let clk_val = ((c_b >> clk_bit) & 1) != 0;
                        if !prev_clk && clk_val {
                            req |= if ((c_b >> dio_bit) & 1) != 0 {
                                1 << b
                            } else {
                                0
                            };
                            prev_clk = clk_val;
                            break;
                        }
                        prev_clk = clk_val;
                    }
                    if !ok {
                        break;
                    }
                }
                if !ok {
                    break;
                }
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
                        if i >= len {
                            break;
                        }
                        let c_b = samples[i.min(len - 1)];
                        let clk_val = ((c_b >> dio_bit) & 1) != 0; // placeholder dummy: parity still
                        if !prev_clk && clk_val {
                            break;
                        }
                    }
                    let mut ack: u8 = 0;
                    for b in 0..3u8 {
                        while i + 1 < len {
                            i += 1;
                            if i >= len {
                                break;
                            }
                            let c_b = samples[i.min(len - 1)];
                            let clk_val = ((c_b >> clk_bit) & 1) != 0;
                            if !prev_clk && clk_val {
                                ack |= (((c_b >> dio_bit) & 1) as u8) << b;
                                break;
                            }
                            let _ = clk_val;
                        }
                    }
                    if ack == 1 && r_nw == 1 {
                        result.data_reads_ok += 1;
                    }
                    if ack == 4 {
                        result.fault_acks += 1;
                    }
                }
            }
            prev_clk = cur_clk;
            i += 1;
        } else {
            prev_clk = cur_clk;
            i += 1;
        }
    }
    let _ = (
        sample_rate,
        SwdReq {
            ap_nap: false,
            read: false,
            addr: 0,
            parity_ok: true,
        },
        SwdAck::Ok,
    );
    result
}

pub const SWD_TEST_SAMPLE: [u8; 45] = [
    0b01000001, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00010000, 0b00000000,
    0b00100001, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000, 0b00000000,
    0b00000000, 0b00000000, 0b00000000,
];

pub const SWD_KAT: [u8; 40] = {
    let mut a = [0u8; 40];
    let mut i = 0usize;
    while i < 40 {
        a[i] = ((i & 1) as u8) | 0b10; // even=0, odd=1 clock; bit1 stays 1
        i += 1;
    }
    a
};
