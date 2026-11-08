use agentla_analyzer::ols::*;
use agentla_analyzer::stream::*;

#[test]
fn test_ols_1byte_commands() {
    let mut parser = OlsParser::new();

    assert_eq!(parser.process_byte(CMD_RESET), Some(OlsCommand::Reset));
    assert_eq!(parser.process_byte(CMD_RUN), Some(OlsCommand::Run));
    assert_eq!(parser.process_byte(CMD_ID), Some(OlsCommand::QueryId));
    assert_eq!(parser.process_byte(CMD_METADATA), Some(OlsCommand::QueryMetadata));
}

#[test]
fn test_ols_5byte_set_divider() {
    let mut parser = OlsParser::new();

    // 0x80 CMD_SET_DIVIDER followed by 4 bytes (little-endian divider)
    // Divider = 99 -> sample_rate = 100_000_000 / (99 + 1) = 1_000_000 Hz (1 MHz)
    assert_eq!(parser.process_byte(CMD_SET_DIVIDER), None);
    assert_eq!(parser.process_byte(99), None);
    assert_eq!(parser.process_byte(0), None);
    assert_eq!(parser.process_byte(0), None);
    let cmd = parser.process_byte(0);

    assert_eq!(
        cmd,
        Some(OlsCommand::SetDivider {
            divider: 99,
            sample_rate_hz: 1_000_000,
        })
    );
}

#[test]
fn test_ols_metadata_tlv() {
    let mut buf = [0u8; 256];
    let len = generate_ols_metadata(&mut buf);
    assert!(len > 20);

    // First TLV should be TLV_DEVICE_NAME (0x01)
    assert_eq!(buf[0], TLV_DEVICE_NAME);
    // Find "AgentLA" in the metadata binary slice
    assert!(buf[..len].windows(7).any(|w| w == b"AgentLA"));
    assert!(buf[..len].windows(11).any(|w| w == b"v1.0.0-rust"));

    // Last byte must be TLV_END (0x00)
    assert_eq!(buf[len - 1], TLV_END);
}

#[test]
fn test_sump_rle_encode_8bit() {
    // 5 consecutive 0x0A samples, then 1 0x0B sample, then 3 0x0C samples
    let samples = [0x0A, 0x0A, 0x0A, 0x0A, 0x0A, 0x0B, 0x0C, 0x0C, 0x0C];
    let mut encoded = [0u8; 32];
    let enc_len = sump_rle_encode_8bit(&samples, &mut encoded);

    // First sample: 0x0A (bit 7 = 0)
    assert_eq!(encoded[0], 0x0A);
    // Repeat count: 4 more -> 0x80 | 4 = 0x84
    assert_eq!(encoded[1], 0x84);
    // Next sample: 0x0B
    assert_eq!(encoded[2], 0x0B);
    // Next sample: 0x0C
    assert_eq!(encoded[3], 0x0C);
    // Repeat count: 2 more -> 0x80 | 2 = 0x82
    assert_eq!(encoded[4], 0x82);
    assert_eq!(enc_len, 5);
}

#[test]
fn test_spsc_ring_buffer_push_pop_overflow() {
    let ring = SpscStreamBuffer::<256>::new();
    assert_eq!(ring.available_to_read(), 0);

    // Push frame 1
    let payload1 = [1, 2, 3, 4];
    assert!(ring.push_frame(FLAG_FORMAT_BIT_DELTA, 8, &payload1));
    assert_eq!(ring.overflow_count(), 0);

    // Read back frame
    let mut out = [0u8; 128];
    let read_len = ring.pop_bytes(&mut out);
    assert_eq!(read_len, StreamFrameHeader::SIZE + 4);

    let hdr = StreamFrameHeader::from_bytes(out[..8].try_into().unwrap()).expect("valid header");
    assert_eq!(hdr.seq, 0);
    assert_eq!(hdr.flags, FLAG_FORMAT_BIT_DELTA);
    assert_eq!(hdr.channels, 8);
    assert_eq!(hdr.payload_len, 4);
    assert_eq!(&out[8..12], &payload1);

    // Fill buffer with three 70-byte frames to trigger overflow (256 cap)
    let large_payload = [0xAA; 70];
    assert!(ring.push_frame(0, 8, &large_payload)); // Frame 1: 78B
    assert!(ring.push_frame(0, 8, &large_payload)); // Frame 2: 78B (156B total)
    assert!(ring.push_frame(0, 8, &large_payload)); // Frame 3: 78B (234B total)
    // 4th push must fail and trigger overflow
    let pushed = ring.push_frame(0, 8, &large_payload);
    assert!(!pushed);
    assert_eq!(ring.overflow_count(), 1);

    // Pop everything in buffer
    while ring.available_to_read() > 0 {
        let _ = ring.pop_bytes(&mut out);
    }

    // Now push a new frame; it should carry FLAG_OVERFLOW
    assert!(ring.push_frame(0, 8, &[9, 9]));
    let n = ring.pop_bytes(&mut out);
    assert_eq!(n, StreamFrameHeader::SIZE + 2);
    let hdr2 = StreamFrameHeader::from_bytes(out[..8].try_into().unwrap()).expect("valid header");
    assert_eq!(hdr2.flags & FLAG_OVERFLOW, FLAG_OVERFLOW);

    // Test pop_payload stripping header
    let payload = [0x11, 0x22, 0x33, 0x44, 0x55];
    assert!(ring.push_frame(0, 8, &payload));
    let mut stripped = [0u8; 16];
    let payload_len = ring.pop_payload(&mut stripped);
    assert_eq!(payload_len, 5);
    assert_eq!(&stripped[..5], &payload);
}
