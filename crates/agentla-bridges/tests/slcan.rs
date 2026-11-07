use agentla_bridges::can::CanFrame;
use agentla_bridges::slcan::*;
use embedded_can::Frame;

#[test]
fn test_speed_table() {
    assert_eq!(SlcanSpeed::S0.bitrate(), 10_000);
    assert_eq!(SlcanSpeed::S1.bitrate(), 20_000);
    assert_eq!(SlcanSpeed::S2.bitrate(), 50_000);
    assert_eq!(SlcanSpeed::S3.bitrate(), 100_000);
    assert_eq!(SlcanSpeed::S4.bitrate(), 125_000);
    assert_eq!(SlcanSpeed::S5.bitrate(), 250_000);
    assert_eq!(SlcanSpeed::S6.bitrate(), 500_000);
    assert_eq!(SlcanSpeed::S7.bitrate(), 800_000);
    assert_eq!(SlcanSpeed::S8.bitrate(), 1_000_000);

    assert_eq!(SlcanSpeed::from_baud(500_000), Some(SlcanSpeed::S6));
    assert_eq!(SlcanSpeed::from_index(5), Some(SlcanSpeed::S5));
}

#[test]
fn test_parse_standard_transmit_spec_scenario() {
    // WHEN receiving ASCII string t123411223344\r
    let cmd = parse_command("t123411223344\r").expect("parse standard frame");

    // THEN the parser returns a standard CAN frame with ID 0x123, DLC 4, and payload [0x11, 0x22, 0x33, 0x44]
    match cmd {
        SlcanCommand::Transmit(frame) => {
            assert!(!frame.is_extended());
            assert!(!frame.is_remote_frame());
            assert_eq!(frame.raw_id(), 0x123);
            assert_eq!(frame.dlc(), 4);
            assert_eq!(frame.data(), &[0x11, 0x22, 0x33, 0x44]);
        }
        other => panic!("expected Transmit command, got {:?}", other),
    }
}

#[test]
fn test_format_extended_frame_spec_scenario() {
    // WHEN serializing a received extended CAN frame with ID 0x18DAF110, DLC 2, and payload [0x02, 0x01]
    let frame = CanFrame::new_extended(0x18DAF110, &[0x02, 0x01]).expect("create extended frame");

    // THEN the formatter generates T18DAF11020201\r
    let formatted = format_frame_string(&frame).expect("format frame");
    assert_eq!(formatted.as_str(), "T18DAF11020201\r");
}

#[test]
fn test_format_standard_frame() {
    let frame = CanFrame::new_standard(0x123, &[0x11, 0x22, 0x33, 0x44]).expect("create standard frame");
    let formatted = format_frame_string(&frame).expect("format frame");
    assert_eq!(formatted.as_str(), "t123411223344\r");
}

#[test]
fn test_parse_extended_transmit() {
    let cmd = parse_command("T18DAF11020201\r\n").expect("parse extended frame");
    match cmd {
        SlcanCommand::Transmit(frame) => {
            assert!(frame.is_extended());
            assert!(!frame.is_remote_frame());
            assert_eq!(frame.raw_id(), 0x18DAF110);
            assert_eq!(frame.dlc(), 2);
            assert_eq!(frame.data(), &[0x02, 0x01]);
        }
        other => panic!("expected Transmit command, got {:?}", other),
    }
}

#[test]
fn test_parse_remote_frames() {
    let cmd = parse_command("r1230\r").expect("parse standard rtr");
    match cmd {
        SlcanCommand::Transmit(frame) => {
            assert!(!frame.is_extended());
            assert!(frame.is_remote_frame());
            assert_eq!(frame.raw_id(), 0x123);
            assert_eq!(frame.dlc(), 0);
        }
        other => panic!("expected Transmit command, got {:?}", other),
    }

    let cmd_ext = parse_command("R18DAF1104\r").expect("parse extended rtr");
    match cmd_ext {
        SlcanCommand::Transmit(frame) => {
            assert!(frame.is_extended());
            assert!(frame.is_remote_frame());
            assert_eq!(frame.raw_id(), 0x18DAF110);
            assert_eq!(frame.dlc(), 4);
        }
        other => panic!("expected Transmit command, got {:?}", other),
    }
}

#[test]
fn test_slcan_engine_lifecycle() {
    let mut engine = SlcanEngine::new();
    assert!(!engine.is_open());

    assert_eq!(engine.process_line("S6").as_str(), "OK");
    assert_eq!(engine.speed, SlcanSpeed::S6);

    // Frame while closed should fail
    assert_eq!(engine.process_line("t123411223344").as_str(), "ERR_CLOSED");
    assert_eq!(engine.tx_count(), 0);

    // Open channel
    assert_eq!(engine.process_line("O").as_str(), "OK");
    assert!(engine.is_open());

    // Frame while open succeeds
    assert_eq!(engine.process_line("t123411223344").as_str(), "z");
    assert_eq!(engine.tx_count(), 1);

    // Close channel
    assert_eq!(engine.process_line("C").as_str(), "OK");
    assert!(!engine.is_open());
}
