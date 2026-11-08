use agentla_analyzer::decoders::event::{DecodedEvent, ProtocolKind};
use agentla_analyzer::decoders::matrix::{HardwareTriggerMatrix, HardwareTriggerRule};
use agentla_analyzer::decoders::uart::{Parity, UartConfig, UartDecoder};

#[test]
fn test_uart_stream_decode_8n1() {
    // 1 MHz clock, 100_000 baud -> 10 ticks per bit
    let cfg = UartConfig {
        rx_pin: 0,
        baud_rate: 100_000,
        data_bits: 8,
        parity: Parity::None,
        stop_bits: 1,
    };
    let mut decoder = UartDecoder::new(cfg, 1_000_000);

    // Byte 0x55 = 0b01010101
    // UART transmission order: Start (0), Bit0 (1), Bit1 (0), Bit2 (1), Bit3 (0), Bit4 (1), Bit5 (0), Bit6 (1), Bit7 (0), Stop (1)
    let bits = [0, 1, 0, 1, 0, 1, 0, 1, 0, 1];
    let mut current_tick = 0u32;
    let mut event_out = None;

    for &bit in &bits {
        let pin_val = if bit == 1 { 1u32 } else { 0u32 };
        for _ in 0..10 {
            current_tick += 1;
            if let Some(ev) = decoder.step(current_tick, pin_val) {
                event_out = Some(ev);
            }
        }
    }

    assert!(event_out.is_some(), "Expected decoded UART event");
    if let Some(DecodedEvent::UartByte { byte, parity_err, framing_err, .. }) = event_out {
        assert_eq!(byte, 0x55);
        assert!(!parity_err);
        assert!(!framing_err);
    } else {
        panic!("Unexpected event type: {:?}", event_out);
    }
}

#[test]
fn test_uart_framing_error() {
    let cfg = UartConfig {
        rx_pin: 0,
        baud_rate: 100_000,
        data_bits: 8,
        parity: Parity::None,
        stop_bits: 1,
    };
    let mut decoder = UartDecoder::new(cfg, 1_000_000);

    // Stop bit held LOW (0) instead of HIGH (1) -> framing error
    let bits = [0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut current_tick = 0u32;
    let mut event_out = None;

    for &bit in &bits {
        let pin_val = if bit == 1 { 1u32 } else { 0u32 };
        for _ in 0..10 {
            current_tick += 1;
            if let Some(ev) = decoder.step(current_tick, pin_val) {
                event_out = Some(ev);
            }
        }
    }

    assert!(event_out.is_some());
    if let Some(DecodedEvent::UartByte { byte, framing_err, .. }) = event_out {
        assert_eq!(byte, 0x01);
        assert!(framing_err, "Expected framing error on low stop bit");
    }
}

#[test]
fn test_hardware_trigger_matrix_i2c_nack() {
    let mut matrix = HardwareTriggerMatrix::new();
    let rule = HardwareTriggerRule {
        rule_id: 1,
        enabled: true,
        bus: ProtocolKind::I2c,
        match_addr: 0x48,
        match_addr_mask: 0x7F,
        match_ack: Some(false), // Match NACK
        match_id: 0,
        match_id_mask: 0,
        match_data_byte0: None,
        match_error: false,
        action_freeze_capture: true,
        action_assert_pin: Some(25),
        action_halt_swd: true,
        action_ws_alert: true,
    };

    assert!(matrix.install_rule(rule).is_ok());

    // 1. Transaction with ACK on 0x48 -> Should NOT match
    let ack_ev = DecodedEvent::I2cTransaction {
        timestamp_us: 100,
        addr: 0x48,
        is_10bit: false,
        is_read: false,
        ack: true,
        data_len: 1,
        data: [0x00; 8],
    };
    assert!(matrix.evaluate(&ack_ev).is_none());

    // 2. Transaction with NACK on different address (0x50) -> Should NOT match
    let diff_addr_ev = DecodedEvent::I2cTransaction {
        timestamp_us: 150,
        addr: 0x50,
        is_10bit: false,
        is_read: false,
        ack: false,
        data_len: 0,
        data: [0x00; 8],
    };
    assert!(matrix.evaluate(&diff_addr_ev).is_none());

    // 3. Transaction with NACK on 0x48 -> MUST MATCH!
    let nack_ev = DecodedEvent::I2cTransaction {
        timestamp_us: 200,
        addr: 0x48,
        is_10bit: false,
        is_read: false,
        ack: false,
        data_len: 0,
        data: [0x00; 8],
    };

    let action = matrix.evaluate(&nack_ev);
    assert!(action.is_some());
    let act = action.unwrap();
    assert_eq!(act.rule_id, 1);
    assert!(act.freeze_capture);
    assert_eq!(act.assert_pin, Some(25));
    assert!(act.halt_swd);
    assert!(act.ws_alert);
}

#[test]
fn test_hardware_trigger_matrix_can_id() {
    let mut matrix = HardwareTriggerMatrix::new();
    let rule = HardwareTriggerRule {
        rule_id: 2,
        enabled: true,
        bus: ProtocolKind::Can,
        match_addr: 0,
        match_addr_mask: 0,
        match_ack: None,
        match_id: 0x7DF,
        match_id_mask: 0x7FF,
        match_data_byte0: Some(0x02),
        match_error: false,
        action_freeze_capture: true,
        action_assert_pin: None,
        action_halt_swd: false,
        action_ws_alert: true,
    };
    assert!(matrix.install_rule(rule).is_ok());

    let can_ev = DecodedEvent::CanFrame {
        timestamp_us: 500,
        id: 0x7DF,
        is_extended: false,
        rtr: false,
        dlc: 8,
        data: [0x02, 0x01, 0x0C, 0, 0, 0, 0, 0],
        crc_ok: true,
    };

    let act = matrix.evaluate(&can_ev);
    assert!(act.is_some());
    assert_eq!(act.unwrap().rule_id, 2);
}

#[test]
fn test_decoded_event_json_serialization() {
    let ev = DecodedEvent::I2cTransaction {
        timestamp_us: 1234,
        addr: 0x48,
        is_10bit: false,
        is_read: false,
        ack: true,
        data_len: 2,
        data: [0x01, 0xA5, 0, 0, 0, 0, 0, 0],
    };

    let mut buf = [0u8; 128];
    let len = ev.format_json(&mut buf);
    let json_str = std::str::from_utf8(&buf[..len]).unwrap();
    assert!(json_str.contains(r#""bus":"i2c""#));
    assert!(json_str.contains(r#""addr":72"#));
    assert!(json_str.contains(r#""ack":true"#));
    assert!(json_str.contains(r#""01""#));
    assert!(json_str.contains(r#""A5""#));
}
