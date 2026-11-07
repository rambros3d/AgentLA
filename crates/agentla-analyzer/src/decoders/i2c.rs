//! I2C transaction decoder for bus sniffer and sample decoding.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct I2cDecodedTransaction {
    pub addr: u8,
    pub is_read: bool,
    pub ack: bool,
    pub data_len: u8,
    pub data: [u8; 8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum I2cState {
    Idle,
    AddrBits(u8, u8),
    AddrAck(u8),
    DataBits(u8, u8),
    DataAck,
}

pub fn i2c_decode_transactions(
    samples: &[u8],
    scl_bit: u8,
    sda_bit: u8,
    out: &mut heapless::Vec<I2cDecodedTransaction, 32>,
) -> usize {
    if samples.is_empty() {
        return 0;
    }
    let mut total_transactions = 0usize;
    let mut state = I2cState::Idle;
    let mut current_tx = I2cDecodedTransaction {
        addr: 0,
        is_read: false,
        ack: false,
        data_len: 0,
        data: [0; 8],
    };

    let mut prev_scl = ((samples[0] >> scl_bit) & 1) != 0;
    let mut prev_sda = ((samples[0] >> sda_bit) & 1) != 0;

    for &b in &samples[1..] {
        let cur_scl = ((b >> scl_bit) & 1) != 0;
        let cur_sda = ((b >> sda_bit) & 1) != 0;

        // START / Repeated START detection
        if prev_scl && cur_scl && prev_sda && !cur_sda {
            if state != I2cState::Idle && (current_tx.addr != 0 || current_tx.data_len > 0) {
                total_transactions += 1;
                let _ = out.push(current_tx);
                current_tx = I2cDecodedTransaction {
                    addr: 0,
                    is_read: false,
                    ack: false,
                    data_len: 0,
                    data: [0; 8],
                };
            }
            state = I2cState::AddrBits(0, 0);
            prev_scl = cur_scl;
            prev_sda = cur_sda;
            continue;
        }

        // STOP condition detection
        if prev_scl && cur_scl && !prev_sda && cur_sda {
            if state != I2cState::Idle && (current_tx.addr != 0 || current_tx.data_len > 0) {
                total_transactions += 1;
                let _ = out.push(current_tx);
                current_tx = I2cDecodedTransaction {
                    addr: 0,
                    is_read: false,
                    ack: false,
                    data_len: 0,
                    data: [0; 8],
                };
            }
            state = I2cState::Idle;
            prev_scl = cur_scl;
            prev_sda = cur_sda;
            continue;
        }

        // SCL rising edge: latch bit
        if !prev_scl && cur_scl {
            let bit = cur_sda as u8;
            match state {
                I2cState::Idle => {}
                I2cState::AddrBits(accum, count) => {
                    let new_accum = (accum << 1) | bit;
                    if count + 1 == 8 {
                        current_tx.addr = new_accum >> 1;
                        current_tx.is_read = (new_accum & 1) != 0;
                        state = I2cState::AddrAck(new_accum);
                    } else {
                        state = I2cState::AddrBits(new_accum, count + 1);
                    }
                }
                I2cState::AddrAck(_addr_byte) => {
                    current_tx.ack = bit == 0;
                    state = I2cState::DataBits(0, 0);
                }
                I2cState::DataBits(accum, count) => {
                    let new_accum = (accum << 1) | bit;
                    if count + 1 == 8 {
                        if (current_tx.data_len as usize) < current_tx.data.len() {
                            current_tx.data[current_tx.data_len as usize] = new_accum;
                            current_tx.data_len += 1;
                        }
                        state = I2cState::DataAck;
                    } else {
                        state = I2cState::DataBits(new_accum, count + 1);
                    }
                }
                I2cState::DataAck => {
                    current_tx.ack = bit == 0;
                    state = I2cState::DataBits(0, 0);
                }
            }
        }

        prev_scl = cur_scl;
        prev_sda = cur_sda;
    }

    if state != I2cState::Idle && (current_tx.addr != 0 || current_tx.data_len > 0) {
        total_transactions += 1;
        let _ = out.push(current_tx);
    }

    total_transactions
}
