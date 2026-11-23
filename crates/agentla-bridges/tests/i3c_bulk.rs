use agentla_bridges::i3c_bulk::*;

#[test]
fn test_i3c_header_serialization_roundtrip() {
    let header = I3cBulkHeader::new(I3C_OP_PRIVATE_WRITE, 42, 0x08, 16);
    let mut buf = [0u8; I3C_HEADER_SIZE];
    assert!(header.serialize(&mut buf));

    let parsed = I3cBulkHeader::parse(&buf).expect("header parse failed");
    assert_eq!(parsed.magic, I3C_BULK_MAGIC);
    assert_eq!(parsed.opcode, I3C_OP_PRIVATE_WRITE);
    assert_eq!(parsed.seq, 42);
    assert_eq!(parsed.addr, 0x08);
    assert_eq!(parsed.payload_len, 16);
    assert_eq!(parsed.status, I3C_STATUS_OK);
}

#[test]
fn test_i3c_frame_parse_and_serialize() {
    let payload = [0x11, 0x22, 0x33, 0x44];
    let header = I3cBulkHeader::new(I3C_OP_ENTDAA, 1, 0x7E, payload.len() as u16);
    let frame = I3cBulkFrame::new(header, &payload);

    let mut buf = [0u8; 32];
    let written = frame.serialize(&mut buf).expect("frame serialize failed");
    assert_eq!(written, I3C_HEADER_SIZE + payload.len());

    let parsed = I3cBulkFrame::parse(&buf[..written]).expect("frame parse failed");
    assert_eq!(parsed.header.opcode, I3C_OP_ENTDAA);
    assert_eq!(parsed.header.seq, 1);
    assert_eq!(parsed.header.addr, 0x7E);
    assert_eq!(parsed.payload, &payload);
}

#[test]
fn test_i3c_make_response() {
    let mut resp_buf = [0u8; 64];
    let rx_data = [0xA0, 0xA1, 0xA2];
    let len = I3cBulkFrame::make_response(
        7,
        I3C_OP_PRIVATE_READ,
        I3C_STATUS_OK,
        0x08,
        &rx_data,
        &mut resp_buf,
    ).expect("make response failed");

    assert_eq!(len, I3C_HEADER_SIZE + rx_data.len());
    let parsed = I3cBulkFrame::parse(&resp_buf[..len]).expect("parse response failed");
    assert_eq!(parsed.header.seq, 7);
    assert_eq!(parsed.header.status, I3C_STATUS_OK);
    assert_eq!(parsed.payload, &rx_data);
}

#[test]
fn test_i3c_invalid_magic_rejected() {
    let mut bad_buf = [0u8; I3C_HEADER_SIZE];
    bad_buf[0] = 0xFF; // Invalid magic
    assert!(I3cBulkHeader::parse(&bad_buf).is_none());
}
