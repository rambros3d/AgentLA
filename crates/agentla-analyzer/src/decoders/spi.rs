//! SPI transaction decoder for bus sniffer and sample decoding.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SpiDecodedTransaction {
    pub data_len: u8,
    pub mosi: [u8; 8],
    pub miso: [u8; 8],
}

pub fn spi_decode_transactions(
    samples: &[u8],
    sck_bit: u8,
    mosi_bit: u8,
    miso_bit: u8,
    cs_bit: u8,
    out: &mut heapless::Vec<SpiDecodedTransaction, 32>,
) -> usize {
    if samples.is_empty() {
        return 0;
    }
    let mut total_transactions = 0usize;
    let mut current_tx = SpiDecodedTransaction {
        data_len: 0,
        mosi: [0; 8],
        miso: [0; 8],
    };

    let mut prev_sck = ((samples[0] >> sck_bit) & 1) != 0;
    let mut prev_cs = ((samples[0] >> cs_bit) & 1) != 0;
    let mut bit_count = 0u8;
    let mut mosi_accum = 0u8;
    let mut miso_accum = 0u8;

    for &b in &samples[1..] {
        let cur_sck = ((b >> sck_bit) & 1) != 0;
        let cur_mosi = ((b >> mosi_bit) & 1) != 0;
        let cur_miso = ((b >> miso_bit) & 1) != 0;
        let cur_cs = ((b >> cs_bit) & 1) != 0;

        // CS deasserted (rising edge to high): finish frame
        if !prev_cs && cur_cs {
            if current_tx.data_len > 0 {
                total_transactions += 1;
                let _ = out.push(current_tx);
                current_tx = SpiDecodedTransaction {
                    data_len: 0,
                    mosi: [0; 8],
                    miso: [0; 8],
                };
            }
            bit_count = 0;
            mosi_accum = 0;
            miso_accum = 0;
        }

        // Active CS (CS low)
        if !cur_cs {
            // SCK rising edge (Mode 0)
            if !prev_sck && cur_sck {
                mosi_accum = (mosi_accum << 1) | (cur_mosi as u8);
                miso_accum = (miso_accum << 1) | (cur_miso as u8);
                bit_count += 1;
                if bit_count == 8 {
                    if (current_tx.data_len as usize) < current_tx.mosi.len() {
                        current_tx.mosi[current_tx.data_len as usize] = mosi_accum;
                        current_tx.miso[current_tx.data_len as usize] = miso_accum;
                        current_tx.data_len += 1;
                    }
                    bit_count = 0;
                    mosi_accum = 0;
                    miso_accum = 0;
                }
            }
        }

        prev_sck = cur_sck;
        prev_cs = cur_cs;
    }

    if current_tx.data_len > 0 {
        total_transactions += 1;
        let _ = out.push(current_tx);
    }

    total_transactions
}
