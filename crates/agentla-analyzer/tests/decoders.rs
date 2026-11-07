use agentla_analyzer::decoders::*;
use agentla_analyzer::trigger::{falling_edge, rising_edge};
use agentla_analyzer::vcd::generate_vcd;

#[test]
fn test_swd_decoder_kat() {
    let res = swd_decode(&SWD_KAT, 1_000_000, 0, 1, SWD_KAT.len());
    // SWD KAT has alternating clock on bit 0 and data on bit 1
    // Verify that swd_decode processes the samples without panicking
    let _ = res.request_count;
}

#[test]
fn test_swd_decoder_sample() {
    let res = swd_decode(
        &SWD_TEST_SAMPLE,
        1_000_000,
        0,
        1,
        SWD_TEST_SAMPLE.len(),
    );
    let _ = res.request_count;
}

#[test]
fn test_i2c_decoder_synthetic() {
    // SCL = bit 1, SDA = bit 0
    // Idle bus: SCL=1, SDA=1 (0b11 = 3)
    let mut samples = Vec::new();
    for _ in 0..5 {
        samples.push(0b11u8);
    }

    // START condition: SCL remains 1, SDA goes 1 -> 0 (0b10 = 2)
    samples.push(0b10);

    // Send 8 bits of address 0x42 (write: 0x42 << 1 | 0 = 0x84 = 0b1000_0100)
    let addr_byte = 0x84u8;
    for bit_idx in (0..8).rev() {
        let bit = (addr_byte >> bit_idx) & 1;
        // SCL low: set SDA
        samples.push(bit);
        // SCL high: sample SDA
        samples.push(0b10 | bit);
        // SCL low
        samples.push(bit);
    }

    // ACK from slave: SDA low (0) while SCL high (0b10)
    samples.push(0b00);
    samples.push(0b10);
    samples.push(0b00);

    // Data byte: 0x55 (0b0101_0101)
    let data_byte = 0x55u8;
    for bit_idx in (0..8).rev() {
        let bit = (data_byte >> bit_idx) & 1;
        samples.push(bit);
        samples.push(0b10 | bit);
        samples.push(bit);
    }

    // ACK: SDA low
    samples.push(0b00);
    samples.push(0b10);
    samples.push(0b00);

    // STOP condition: SCL high while SDA transitions 0 -> 1
    samples.push(0b10); // SCL high, SDA 0
    samples.push(0b11); // SCL high, SDA 1
    samples.push(0b11);

    let mut out = heapless::Vec::<I2cDecodedTransaction, 32>::new();
    let count = i2c_decode_transactions(&samples, 1, 0, &mut out);

    assert_eq!(count, 1);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].addr, 0x42);
    assert!(!out[0].is_read);
    assert!(out[0].ack);
    assert_eq!(out[0].data_len, 1);
    assert_eq!(out[0].data[0], 0x55);
}

#[test]
fn test_spi_decoder_synthetic() {
    // SCK = bit 0, MOSI = bit 1, MISO = bit 2, CS = bit 3
    // Idle: CS=1 (bit 3 high => 0b1000 = 8)
    let mut samples = Vec::new();
    for _ in 0..5 {
        samples.push(0b1000u8);
    }

    // CS assert low (0b0000)
    samples.push(0b0000);

    // Clock out byte MOSI=0xA5 (0b1010_0101), MISO=0x3C (0b0011_1100)
    let mosi_byte = 0xA5u8;
    let miso_byte = 0x3Cu8;
    for bit_idx in (0..8).rev() {
        let mosi_bit = (mosi_byte >> bit_idx) & 1;
        let miso_bit = (miso_byte >> bit_idx) & 1;
        let data_val = (mosi_bit << 1) | (miso_bit << 2);

        // SCK low
        samples.push(data_val);
        // SCK high (bit 0 = 1) -> sample
        samples.push(data_val | 1);
    }

    // Return SCK low
    samples.push(0b0000);
    // CS deassert high (bit 3 = 1)
    samples.push(0b1000);
    samples.push(0b1000);

    let mut out = heapless::Vec::<SpiDecodedTransaction, 32>::new();
    let count = spi_decode_transactions(&samples, 0, 1, 2, 3, &mut out);

    assert_eq!(count, 1);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].data_len, 1);
    assert_eq!(out[0].mosi[0], 0xA5);
    assert_eq!(out[0].miso[0], 0x3C);
}

#[test]
fn test_trigger_edges() {
    let samples = [0b00u8, 0b01, 0b01, 0b00, 0b01, 0b00];
    let rising = rising_edge(&samples, 0);
    assert_eq!(rising, 2);

    let falling = falling_edge(&samples, 0);
    assert_eq!(falling, 2);
}

#[test]
fn test_vcd_generation() {
    let samples = [0x00u32, 0x01, 0x01, 0x03, 0x00];
    let mut vcd_str = heapless::String::<2048>::new();
    generate_vcd(&samples, &mut vcd_str);

    assert!(vcd_str.contains("$timescale 10ns $end"));
    assert!(vcd_str.contains("$var wire 1 ! ch0 $end"));
    assert!(vcd_str.contains("$enddefinitions $end"));
    assert!(vcd_str.contains("#0"));
}
