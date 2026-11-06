## 1. Protocol Data Structures & Handlers
- [x] 1.1 Create `firmware-rs/src/bridge/can_gsusb.rs` with `gs_usb` constants, request enums, `GsHostFrame`, `GsDeviceConfig`, `GsDeviceBtConst`, and pin lifecycle state.
- [x] 1.2 Create `firmware-rs/src/bridge/i2c_tinyusb.rs` with `i2c-tiny-usb` commands, capability flags, status constants, and integration with `crate::bridge::i2c`.

## 2. Embassy USB Composite Layout & Task Integration
- [x] 2.1 Update `firmware-rs/src/usb.rs` to declare the `gs_usb` bulk interface (1 Bulk IN, 1 Bulk OUT) and `i2c-tiny-usb` interface.
- [x] 2.2 Implement `embassy_usb::Handler` for vendor control transfers (`gs_usb` Brets and `i2c-tiny-usb` commands).
- [x] 2.3 Implement and spawn `gs_usb_task` to handle Bulk frame read, echo loopback, and frame transmission.

## 3. Web & Telemetry Integration
- [x] 3.1 Expose CAN adapter endpoints in `firmware-rs/src/web.rs` (`/api/v1/bridge/can`, `/api/v1/bridge/can/termination`).
- [x] 3.2 Update `/api/v1/bridge` and `/api/board/manifest` to report native SocketCAN and I2C adapter capabilities.

## 4. Verification on Hardware
- [x] 4.1 Recompile `firmware-rs` release build and generate UF2.
- [x] 4.2 Flash physical RP2350 probe and verify USB enumeration (`lsusb`, interface list).
- [x] 4.3 Verify EP0 control requests and REST telemetry endpoints.
- [x] 4.4 Update `FUTURE.md` and remove completed Section 2 item.
