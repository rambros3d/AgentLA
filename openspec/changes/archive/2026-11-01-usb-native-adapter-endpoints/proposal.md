## Why

Currently, AgentLA's USB composite descriptor exposes network (WebUI), two CDC serial ports, and CMSIS-DAP HID. To interact with CAN or I2C buses, users must either use the WebUI or route commands through ASCII CLI commands or JSON-RPC.

Operating systems like Linux already possess native in-kernel drivers for USB adapters:
- **`gs_usb`** (`drivers/net/can/usb/gs_usb.c`) enables plug-and-play **SocketCAN** (`can0`), supported directly by `candump`, `cansend`, Wireshark, and SavvyCAN without installing third-party drivers.
- **`i2c-tiny-usb`** (`drivers/i2c/busses/i2c-tiny-usb.c`) exposes **`/dev/i2c-X`**, allowing standard tools like `i2cdetect` and Python `smbus2` to talk to target sensors directly.

By adding these interfaces to the pre-enumerated USB composite descriptor (Approach 1) alongside dedicated transceiver pin definitions (CAN on GP18/19, RS-485 on GP12–14), workstation tools can communicate with buses natively while physical pins remain high-impedance until claimed.

## What Changes

- **USB Composite Descriptor Update (`firmware/src/programmer/usb_hid.c` & `tusb_config.h`)**:
  - Add `gs_usb` Vendor Interface (1 Bulk IN, 1 Bulk OUT) for native SocketCAN.
  - Add `i2c-tiny-usb` Endpoint 0 vendor request handling for native Linux I2C.
  - Utilize remaining free hardware endpoints on RP2350 (7 IN / 11 OUT still available).
- **Standby & Dormant Lifecycle Management**:
  - Interfaces enumerate on first USB plug-in.
  - If pins are unconfigured, physical pins remain tri-stated (Hi-Z); `can0` remains in `DOWN` state and I2C requests return `-ENODEV` or trigger auto-claim.
- **Fixed Hardware Transceiver Pin Mapping**:
  - Reserve **GPIO 18 & 19** for CAN Transceiver (`CAN_TX`, `CAN_RX`) driven by PIO-CAN.
  - Reserve **GPIO 22** for software-switchable $120\,\Omega$ CAN bus termination.
  - Reserve **GPIO 12, 13 & 14** for RS-485 Transceiver (`TX`, `RX`, and automatic direction control `DE_RE`).
- **WebUI & Pin Registry Integration**:
  - Expose transceiver statuses and bus state pills on the 30-Pin Dynamic Matrix and Dashboard.

## Capabilities

### New Capabilities
- `usb-socketcan-gsusb`: Native in-kernel SocketCAN `gs_usb` USB interface exposing `can0` on host workstations.
- `usb-i2c-tiny-usb`: Native in-kernel `i2c-tiny-usb` interface exposing `/dev/i2c-X` on Linux hosts.
- `transceiver-pin-blocks`: Dedicated hardware pin reservations for differential CAN (GP18/19) and RS-485 (GP12–14) transceivers with switchable termination.

### Modified Capabilities
<!-- None: Existing CMSIS-DAP and WebUI endpoints remain unchanged -->

## Impact

- **Firmware**: TinyUSB descriptors in `firmware/src/programmer/usb_hid.c`, `firmware/src/tusb_config.h`, and `firmware/src/board_settings.h`.
- **Operating System Compatibility**:
  - Linux: Instant `can0` and `/dev/i2c-X` support with zero driver installation.
  - Windows: WinUSB / WCID support for CandleLight and Python-CAN.
- **Endpoints**: Consumes 2 IN and 2 OUT endpoints, leaving 5 IN and 9 OUT endpoints free on RP2350.
