# Design: Host-Native USB Kernel Interfaces (gs_usb & i2c-tiny-usb)

## Context

Developers testing and bringing up target hardware with AgentLA frequently need standard OS-level bus access (SocketCAN `can0` and Linux `/dev/i2c-X`). Requiring proprietary software, serial CLI parsers, or HTTP endpoints slows down automated testing scripts (`pytest`, `python-can`, `ros2_can`). Implementing the in-kernel USB protocols provides zero-install native hardware passthrough.

---

## Goals & Non-Goals

### Goals
- Implement `gs_usb` Vendor Interface in the Embassy USB composite descriptor with Bulk IN and Bulk OUT endpoints.
- Handle `gs_usb` EP0 setup requests: `HOST_FORMAT`, `BITTIMING`, `MODE`, `BERR`, `BT_CONST`, `DEVICE_CONFIG`, `SET_TERMINATION`, `GET_TERMINATION`.
- Handle `i2c-tiny-usb` EP0 setup requests: `CMD_ECHO`, `CMD_GET_FUNC`, `CMD_SET_DELAY`, `CMD_GET_STATUS`, `CMD_I2C_IO`.
- Standby / Hi-Z safety lifecycle: keep transceiver pins tri-stated until `can0` is brought UP or I2C transaction is initiated.
- Implement transmission echo loopback on Bulk IN endpoint for `gs_usb` transmit acknowledgments.
- Maintain hardware REST API compatibility for telemetry and web dashboard.

### Non-Goals
- Full hardware CAN controller emulation at 1 Mbps with acceptance filters (this initial milestone focuses on transceiver pin management, loopback/echo, bit timing negotiation, and packet exchange).
- Re-enumerating the USB device (soft detach) on mode changes; all interfaces remain pre-enumerated.

---

## Architecture & Data Flow

### 1. USB Descriptor Layout

The composite device configuration descriptor defines:
- **Interface 0 & 1**: CDC-ACM Target UART Console (EP1 INT IN, EP1 BULK OUT, EP2 BULK IN)
- **Interface 2**: CMSIS-DAP v1 (HID) (EP2 INT OUT, EP3 INT IN)
- **Interface 3**: CMSIS-DAP v2 (Bulk) (EP3 BULK OUT, EP4 BULK IN)
- **Interface 4 & 5**: CDC-NCM Ethernet (EP5 INT IN, EP4 BULK OUT, EP6 BULK IN)
- **Interface 6**: `gs_usb` SocketCAN (Vendor Class `0xFF`, Subclass `0x00`, Protocol `0x00`) (EP5 BULK OUT, EP7 BULK IN)
- **Interface 7**: `i2c-tiny-usb` (Vendor Class `0xFF`, Subclass `0x00`, Protocol `0x00`, string "AgentLA i2c-tiny-usb") (EP0 Control Only)

### 2. Control Transfer Dispatch (`embassy_usb::Handler`)

Control requests on Endpoint 0 with `RequestType::Vendor` are dispatched based on `req.request`:
- **`gs_usb` Brets (0x00–0x0E)**:
  - `GS_USB_BREQ_HOST_FORMAT (0)`: Accepts `0xEFBE0000` / `0x0000BEEF`.
  - `GS_USB_BREQ_BITTIMING (1)`: Parses `prop_seg`, `phase_seg1`, `phase_seg2`, `sjw`, `brp`.
  - `GS_USB_BREQ_MODE (2)`: If `mode == 1` (`GS_CAN_MODE_START`), claims pins and drives transceiver; if `mode == 0` (`GS_CAN_MODE_RESET`), releases pins to Hi-Z.
  - `GS_USB_BREQ_BT_CONST (4)`: Returns `struct gs_device_bt_const` (fclk = 48 MHz).
  - `GS_USB_BREQ_DEVICE_CONFIG (5)`: Returns `struct gs_device_config` (1 channel, sw_ver = 1, hw_ver = 1).
  - `GS_USB_BREQ_SET_TERMINATION (12)`: Controls GPIO 22 termination FET.
  - `GS_USB_BREQ_GET_TERMINATION (13)`: Returns current termination state (0 or 1).

- **`i2c-tiny-usb` Commands (0x00–0x04)**:
  - `CMD_ECHO (0)`: Echoes back `wValue` uint16 (used by Linux kernel probe to verify device presence).
  - `CMD_GET_FUNC (1)`: Returns `0x0EFF0001` (`I2C_FUNC_I2C | I2C_FUNC_SMBUS_EMUL`).
  - `CMD_SET_DELAY (2)`: Adjusts bus clock delay from `wValue`.
  - `CMD_GET_STATUS (3)`: Returns `STATUS_ADDRESS_ACK` (1) or `STATUS_ADDRESS_NAK` (2).
  - `CMD_I2C_IO (4)`: Executes read or write on `addr = wIndex & 0x7F` using `crate::bridge::i2c`.

### 3. SocketCAN Bulk Framing (`gs_host_frame`)

```rust
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GsHostFrame {
    pub echo_id: u32,
    pub can_id: u32,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: u8,
    pub reserved: u8,
    pub data: [u8; 8],
}
```

- When host transmits a frame, `echo_id` is an internal frame identifier assigned by Linux `gs_usb.c`.
- The firmware reads the frame from Bulk OUT, processes the frame, and writes the frame back to Bulk IN with the same `echo_id`. Linux marks the transmission as acknowledged and propagates the packet to local SocketCAN listeners with exact hardware TX timestamp.

---

## Pin Allocation & Hardware Lifecycle

| Pin | Function | Default State (can0 DOWN / idle) | Active State (can0 UP / running) |
|---|---|---|---|
| **GPIO 18** | `CAN_TX` | High-Z (tri-state, input) | SIO OUT driven HIGH (recessive bus state) |
| **GPIO 19** | `CAN_RX` | High-Z (tri-state, input) | SIO IN with weak pull-up |
| **GPIO 22** | `CAN_TERM` | High-Z (termination open) | SIO OUT HIGH (120 Ω engaged) if enabled |
| **GPIO 16** | `I2C_SDA` | High-Z (open-drain float) | Open-drain clock/data pulses |
| **GPIO 17** | `I2C_SCL` | High-Z (open-drain float) | Open-drain clock/data pulses |
