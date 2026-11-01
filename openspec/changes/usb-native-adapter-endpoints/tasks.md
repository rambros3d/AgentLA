## 1. USB Descriptors & Configuration

- [x] 1.1 Update `firmware/src/tusb_config.h` to declare vendor bulk endpoints for SocketCAN `gs_usb`
- [x] 1.2 Update `desc_config` in `firmware/src/programmer/usb_hid.c` with the `gs_usb` interface and `i2c-tiny-usb` endpoint 0 vendor request hooks
- [x] 1.3 Verify endpoint count remains within RP2350 hardware limits (8 IN / 5 OUT used out of 15 available)

## 2. Driverless Protocol Implementations

- [x] 2.1 Implement `gs_usb` packet encoding/decoding and framing loop in `firmware/src/adapter/`
- [x] 2.2 Implement `i2c-tiny-usb` control transfer dispatcher on Endpoint 0
- [x] 2.3 Add standby tri-state logic ensuring pins remain Hi-Z until claimed or brought up by host OS

## 3. Dedicated Transceiver Pin Blocks

- [x] 3.1 Map GPIO 18 (`CAN_TX`), GPIO 19 (`CAN_RX`), and GPIO 22 ($120\,\Omega$ termination) in `firmware/src/board_settings.h`
- [x] 3.2 Map GPIO 12 (`UART0_TX`), GPIO 13 (`UART0_RX`), and GPIO 14 (`RS485_DE`) in `firmware/src/board_settings.h`
- [x] 3.3 Update `docs/pin-allocation.md` to document the dedicated transceiver assignments

## 4. WebUI & Verification

- [x] 4.1 Update the 30-Pin visualizer in `firmware/html/index.html` to display CAN and RS-485 transceiver badges
- [x] 4.2 Compile firmware (`make -C build-work AgentLA`) and verify USB descriptor compilation
- [x] 4.3 Verify Linux kernel host detection of `can0` and `/dev/i2c-X`
