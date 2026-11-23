use agentla_bridges::spi_mpsse::*;

#[test]
fn test_mpsse_initial_state_and_clock() {
    let mut engine = MpsseEngine::new();
    assert_eq!(engine.low_val, 0x08); // CS inactive high
    assert!(!engine.is_cs_asserted());

    // With divisor 1 and 60MHz base: 60M / ((1 + 1) * 2) = 15 MHz
    assert_eq!(engine.clock_hz(), 15_000_000);

    // Enable div by 5: 12M / ((1 + 1) * 2) = 3 MHz
    engine.div_by_5 = true;
    assert_eq!(engine.clock_hz(), 3_000_000);
}

#[test]
fn test_mpsse_bad_command_echo() {
    let mut engine = MpsseEngine::new();
    let stream = [0xAA, 0xAB];
    let mut bad_cmds = Vec::new();

    let consumed = engine.process_stream(&stream, |action| {
        if let MpsseAction::BadCommand(cmd) = action {
            bad_cmds.push(cmd);
        }
    });

    assert_eq!(consumed, 2);
    assert_eq!(bad_cmds, vec![0xAA, 0xAB]);
}

#[test]
fn test_mpsse_gpio_set_and_cs() {
    let mut engine = MpsseEngine::new();
    // CMD_SET_BITS_LOW: val=0x00 (assert CS), dir=0x0B
    let stream = [CMD_SET_BITS_LOW, 0x00, 0x0B];
    let mut low_actions = Vec::new();

    let consumed = engine.process_stream(&stream, |action| {
        if let MpsseAction::GpioSetLow { value, direction } = action {
            low_actions.push((value, direction));
        }
    });

    assert_eq!(consumed, 3);
    assert_eq!(low_actions, vec![(0x00, 0x0B)]);
    assert!(engine.is_cs_asserted());
}

#[test]
fn test_mpsse_divisor_configuration() {
    let mut engine = MpsseEngine::new();
    // CMD_SET_TCK_DIVISOR: low=0x04, high=0x00 -> divisor = 4
    // Clock = 60MHz / ((1 + 4) * 2) = 6 MHz
    let stream = [CMD_SET_TCK_DIVISOR, 0x04, 0x00];
    let mut configured_speeds = Vec::new();

    let consumed = engine.process_stream(&stream, |action| {
        if let MpsseAction::SpeedConfigured(hz) = action {
            configured_speeds.push(hz);
        }
    });

    assert_eq!(consumed, 3);
    assert_eq!(configured_speeds, vec![6_000_000]);
    assert_eq!(engine.divisor, 4);
}

#[test]
fn test_mpsse_data_out_bytes() {
    let mut engine = MpsseEngine::new();
    // CMD_DATA_OUT_BYTES_NEG: length=2 (count=1: low=0x01, high=0x00), data=[0x9F, 0x00]
    let stream = [CMD_DATA_OUT_BYTES_NEG, 0x01, 0x00, 0x9F, 0x00];
    let mut transfers = Vec::new();

    let consumed = engine.process_stream(&stream, |action| {
        if let MpsseAction::SpiTransfer { tx_data, rx_len, mode, lsb_first } = action {
            transfers.push((tx_data.to_vec(), rx_len, mode, lsb_first));
        }
    });

    assert_eq!(consumed, 5);
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].0, vec![0x9F, 0x00]);
    assert_eq!(transfers[0].1, 0); // No read requested
    assert_eq!(transfers[0].2, MpsseSpiMode::Mode0);
    assert!(!transfers[0].3); // MSB first
}

#[test]
fn test_mpsse_full_duplex_transfer() {
    let mut engine = MpsseEngine::new();
    // CMD_DATA_IN_OUT_BYTES_NEG_POS: count=0 (1 byte: low=0, high=0), tx=[0x06]
    let stream = [CMD_DATA_IN_OUT_BYTES_NEG_POS, 0x00, 0x00, 0x06];
    let mut transfers = Vec::new();

    let consumed = engine.process_stream(&stream, |action| {
        if let MpsseAction::SpiTransfer { tx_data, rx_len, mode, lsb_first } = action {
            transfers.push((tx_data.to_vec(), rx_len, mode, lsb_first));
        }
    });

    assert_eq!(consumed, 4);
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers[0].0, vec![0x06]);
    assert_eq!(transfers[0].1, 1); // 1 byte read
    assert_eq!(transfers[0].2, MpsseSpiMode::Mode0);
}
