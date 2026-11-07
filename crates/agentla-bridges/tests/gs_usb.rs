use agentla_bridges::can::CanFrame;
use agentla_bridges::gs_usb::*;
use embedded_can::{Frame, Id};

#[test]
fn test_struct_sizes() {
    assert_eq!(core::mem::size_of::<GsHostConfig>(), 4);
    assert_eq!(core::mem::size_of::<GsDeviceConfig>(), 12);
    assert_eq!(core::mem::size_of::<GsDeviceBtConst>(), 40);
    assert_eq!(core::mem::size_of::<GsDeviceBittiming>(), 20);
    assert_eq!(core::mem::size_of::<GsDeviceMode>(), 8);
    assert_eq!(core::mem::size_of::<GsHostFrame>(), 20);
    assert_eq!(core::mem::size_of::<GsDeviceBerr>(), 8);
}

#[test]
fn test_host_format_negotiation() {
    let features = validate_host_format(GS_CAN_HOST_FORMAT_LITTLE_ENDIAN)
        .expect("little endian format should succeed");
    assert_eq!(
        features,
        GS_CAN_FEATURE_LISTEN_ONLY | GS_CAN_FEATURE_LOOP_BACK | GS_CAN_FEATURE_TRIPLE_SAMPLE
    );

    let features2 = validate_host_format(1).expect("format 1 should succeed");
    assert_eq!(features, features2);

    assert!(validate_host_format(0x12345678).is_err());
}

#[test]
fn test_bittiming_math() {
    let bt = GsDeviceBittiming {
        prop_seg: 5,
        phase_seg1: 5,
        phase_seg2: 5,
        sjw: 1,
        brp: 6,
    };
    let fclk = 48_000_000;
    let bitrate = calculate_bitrate(fclk, &bt).expect("valid bittiming");
    // t_total = 1 + 5 + 5 + 5 = 16
    // 48_000_000 / (6 * 16) = 500_000
    assert_eq!(bitrate, 500_000);
}

#[test]
fn test_frame_packing_unpacking_standard() {
    let can_frame = CanFrame::new_standard(0x123, &[0x11, 0x22, 0x33, 0x44])
        .expect("create standard can frame");
    let host_frame = GsHostFrame::from_can_frame(&can_frame, 100, 0);

    assert_eq!(host_frame.echo_id(), 100);
    assert_eq!(host_frame.channel(), 0);
    assert_eq!(host_frame.can_id(), 0x123);
    assert_eq!(host_frame.can_dlc(), 4);
    assert_eq!(&host_frame.data_bytes()[..4], &[0x11, 0x22, 0x33, 0x44]);

    let unpacked = host_frame.to_can_frame().expect("unpack standard frame");
    assert_eq!(unpacked.raw_id(), 0x123);
    assert!(!unpacked.is_extended());
    assert!(!unpacked.is_remote_frame());
    assert_eq!(unpacked.dlc(), 4);
    assert_eq!(unpacked.data(), &[0x11, 0x22, 0x33, 0x44]);
}

#[test]
fn test_frame_packing_unpacking_extended() {
    let can_frame = CanFrame::new_extended(0x18DAF110, &[0x02, 0x01])
        .expect("create extended can frame");
    let host_frame = GsHostFrame::from_can_frame(&can_frame, 0xFFFF_FFFF, 0);

    assert_eq!(host_frame.echo_id(), 0xFFFF_FFFF);
    assert_eq!(host_frame.can_id(), 0x18DAF110 | GS_CAN_EFF_FLAG);
    assert_eq!(host_frame.can_dlc(), 2);
    assert_eq!(&host_frame.data_bytes()[..2], &[0x02, 0x01]);

    let unpacked = host_frame.to_can_frame().expect("unpack extended frame");
    assert_eq!(unpacked.raw_id(), 0x18DAF110);
    assert!(unpacked.is_extended());
    assert!(!unpacked.is_remote_frame());
    assert_eq!(unpacked.dlc(), 2);
    assert_eq!(unpacked.data(), &[0x02, 0x01]);
}

#[test]
fn test_frame_packing_rtr() {
    let id = embedded_can::StandardId::new(0x456).unwrap();
    let rtr_frame = CanFrame::new_remote(Id::Standard(id), 0).unwrap();
    let host_frame = GsHostFrame::from_can_frame(&rtr_frame, 1, 0);

    assert_eq!(host_frame.can_id(), 0x456 | GS_CAN_RTR_FLAG);
    let unpacked = host_frame.to_can_frame().unwrap();
    assert!(unpacked.is_remote_frame());
    assert_eq!(unpacked.dlc(), 0);
}

#[test]
fn test_engine_control_flow() {
    let mut engine = GsUsbEngine::new(48_000_000);
    assert!(!engine.is_active());

    // 1. Host format
    let host_cfg = GsHostConfig {
        byte_order: GS_CAN_HOST_FORMAT_LITTLE_ENDIAN,
    };
    let cfg_bytes = unsafe {
        core::slice::from_raw_parts(
            &host_cfg as *const _ as *const u8,
            core::mem::size_of::<GsHostConfig>(),
        )
    };
    let action = engine
        .handle_control_out(GS_USB_BREQ_HOST_FORMAT, 0, 0, cfg_bytes)
        .expect("negotiate format")
        .expect("action present");
    assert!(matches!(action, GsControlAction::FormatNegotiated(_)));

    // 2. Bittiming
    let bt = GsDeviceBittiming {
        prop_seg: 5,
        phase_seg1: 5,
        phase_seg2: 5,
        sjw: 1,
        brp: 6,
    };
    let bt_bytes = unsafe {
        core::slice::from_raw_parts(
            &bt as *const _ as *const u8,
            core::mem::size_of::<GsDeviceBittiming>(),
        )
    };
    let action = engine
        .handle_control_out(GS_USB_BREQ_BITTIMING, 0, 0, bt_bytes)
        .expect("bittiming")
        .expect("action present");
    assert_eq!(action, GsControlAction::BittimingUpdated(500_000));
    assert_eq!(engine.bitrate, 500_000);

    // 3. Mode START
    let mode = GsDeviceMode {
        mode: GS_CAN_MODE_START,
        flags: 0,
    };
    let mode_bytes = unsafe {
        core::slice::from_raw_parts(
            &mode as *const _ as *const u8,
            core::mem::size_of::<GsDeviceMode>(),
        )
    };
    let action = engine
        .handle_control_out(GS_USB_BREQ_MODE, 0, 0, mode_bytes)
        .expect("mode start")
        .expect("action present");
    assert_eq!(action, GsControlAction::ModeChange(GS_CAN_MODE_START));
    assert!(engine.is_active());

    // 4. Termination
    let term_action = engine
        .handle_control_out(GS_USB_BREQ_SET_TERMINATION, 1, 0, &[])
        .expect("set term")
        .expect("action");
    assert_eq!(term_action, GsControlAction::TerminationChanged(true));
    assert!(engine.termination);

    // 5. In requests
    let mut buf = [0u8; 64];
    let len = engine
        .handle_control_in(GS_USB_BREQ_GET_TERMINATION, 0, 0, &mut buf)
        .expect("get termination");
    assert_eq!(len, 4);
    assert_eq!(u32::from_le_bytes(buf[..4].try_into().unwrap()), 1);

    let len = engine
        .handle_control_in(GS_USB_BREQ_BT_CONST, 0, 0, &mut buf)
        .expect("get bt const");
    assert_eq!(len, 40);

    let len = engine
        .handle_control_in(GS_USB_BREQ_DEVICE_CONFIG, 0, 0, &mut buf)
        .expect("get dev cfg");
    assert_eq!(len, 12);
}
