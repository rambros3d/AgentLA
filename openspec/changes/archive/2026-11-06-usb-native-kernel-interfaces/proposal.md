## Why

Currently, AgentLA's USB descriptor exposes CDC-ACM (Target UART), CMSIS-DAP v1/v2, and CDC-NCM Ethernet (`10.10.10.10`). While users can interact with I2C and CAN via the DaisyUI Web Desktop or REST API (`/api/v1/bridge/*`), host-side developer tools (such as Linux `candump`, `cansend`, Wireshark, python-can, `i2cdetect`, `i2cget`, `i2cset`) cannot access the physical buses directly as native operating system devices.

Linux provides standard, kernel-level drivers that require zero third-party software installation:
1. **`gs_usb`** (`drivers/net/can/usb/gs_usb.c`): Binds to a USB bulk-endpoint interface and exposes a standard Linux network interface `can0` (`ip link set can0 up type can bitrate 500000`).
2. **`i2c-tiny-usb`** (`drivers/i2c/busses/i2c-tiny-usb.c`): Binds via standard vendor control transfers on Endpoint 0 and exposes `/dev/i2c-X` for plug-and-play I2C bus tools.

By integrating both protocols into AgentLA's Embassy USB composite stack, developers can use native OS tools out of the box while preserving the existing web, REST, and CMSIS-DAP capabilities.

---

## What Changes

1. **USB Stack & Descriptors (`firmware-rs/src/usb.rs`)**:
   - Add a Vendor Interface for `gs_usb` with 1 Bulk IN endpoint (64 bytes) and 1 Bulk OUT endpoint (64 bytes).
   - Add an Interface and Endpoint 0 Vendor Request Handler implementing both `gs_usb` control requests (`BREQ_HOST_FORMAT`, `BREQ_BITTIMING`, `BREQ_MODE`, `BREQ_BT_CONST`, `BREQ_DEVICE_CONFIG`, `BREQ_SET_TERMINATION`) and `i2c-tiny-usb` control requests (`CMD_ECHO`, `CMD_GET_FUNC`, `CMD_SET_DELAY`, `CMD_GET_STATUS`, `CMD_I2C_IO`).
   - Add background Embassy task `gs_usb_task` to handle bulk frame RX/TX and transmission echo loopback.

2. **CAN Adapter Bridge Driver (`firmware-rs/src/bridge/can_gsusb.rs`)**:
   - Manage physical CAN transceiver pins: GPIO 18 (`CAN_TX`), GPIO 19 (`CAN_RX`), and GPIO 22 (`CAN_TERM`).
   - Implement standby lifecycle: tri-state / Hi-Z when `can0` is DOWN (`GS_CAN_MODE_RESET`); claim pins, drive recessive high TX and configure RX with pull-up when `can0` is UP (`GS_CAN_MODE_START`).
   - Software-switchable $120\,\Omega$ termination control via both USB request (`GS_USB_BREQ_SET_TERMINATION`) and REST API.

3. **I2C Tiny USB Bridge Integration (`firmware-rs/src/bridge/i2c_tinyusb.rs` / `firmware-rs/src/bridge/i2c.rs`)**:
   - Connect `CMD_I2C_IO` transfers directly to `hw_i2c_write` / `hw_i2c_read` on flexible pins (GPIO 16 SDA, GPIO 17 SCL).
   - Maintain `last_status` (`STATUS_ADDRESS_ACK` / `STATUS_ADDRESS_NAK`) for kernel error reporting.

4. **REST API & Telemetry (`firmware-rs/src/web.rs`)**:
   - Expose `/api/v1/bridge/can` (GET status, POST termination / mode).
   - Expose `/api/v1/bridge/i2c` (GET status, POST scan / read / write).

---

## Capabilities

### New Capabilities
- `usb-socketcan-gsusb`: Native Linux `gs_usb` protocol implementation providing standard `can0` interface on host workstations.
- `usb-i2c-tiny-usb`: Native Linux `i2c-tiny-usb` protocol implementation exposing `/dev/i2c-X` on Linux hosts.

### Modified Capabilities
- `composite-usb-layout`: Expanded composite descriptor to include vendor CAN interface without disturbing CDC-ACM, CMSIS-DAP, or CDC-NCM Ethernet.
- `pin-ownership`: CAN (GP18/19/22) and I2C (GP16/17) dynamically claimed on demand and returned to tri-state when interface is down.

---

## Impact

- **Zero Breaking Changes**: Existing CDC-ACM UART (`/dev/ttyACM1`), CMSIS-DAP v1/v2, and CDC-NCM Ethernet (`10.10.10.10`) continue to operate uninterrupted.
- **Hardware Resources**: Consumes 1 Bulk IN and 1 Bulk OUT endpoint on RP2350 USB peripheral (leaving 9 IN / 10 OUT endpoints free).
- **Tooling Compatibility**: Works with `iproute2` (`ip link`), `can-utils` (`candump`, `cansend`, `cangen`), Wireshark, python-can, `i2c-tools` (`i2cdetect`, `i2cget`, `i2cset`), and `libgpiod`.
