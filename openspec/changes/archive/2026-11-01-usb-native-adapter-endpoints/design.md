## Context

AgentLA currently exposes a composite USB device containing ECM Network, CDC0 (CLI), CDC1 (Debug Log), and HID (CMSIS-DAP). To allow host software (Wireshark, Python-CAN, SavvyCAN, i2c-tools) to communicate directly with physical buses, AgentLA implements **Approach 1 (Pre-Enumerated In-Kernel OS Drivers)**.

## Goals / Non-Goals

**Goals:**
- Add a pre-enumerated `gs_usb` interface for native Linux SocketCAN (`can0`).
- Add pre-enumerated `i2c-tiny-usb` control transfer handling for Linux `/dev/i2c-X`.
- Map dedicated physical pins for CAN transceivers (GP18/19 with switchable $120\,\Omega$ termination on GP22) and RS-485 transceivers (GP12–14).
- Ensure physical pins remain tri-stated (Hi-Z) when interfaces are unarmed.
- Prevent USB disconnect/reconnect interruptions when toggling adapters.

**Non-Goals:**
- Dynamic USB re-enumeration (soft replug) that resets the network or serial ports.

## Decisions

### 1. Pre-Enumerated Composite Descriptor
- Declare `gs_usb` (Vendor Interface, 1 Bulk IN, 1 Bulk OUT) and `i2c-tiny-usb` in the initial `desc_config`.
- Endpoints allocated:
  - `EDPT_CAN_OUT`: `0x04`
  - `EDPT_CAN_IN`: `0x88`
  - Total endpoints used: 8 IN / 5 OUT (well within the RP2350's 15 IN / 15 OUT limit).

### 2. Standby & Auto-Claim Behavior
- **CAN**: Device initializes with `can0` in the `DOWN` state. When user issues `ip link set can0 up` or claims CAN in the WebUI, PIO-CAN is clocked and transceiver enabled.
- **I2C**: Device responds to `i2c-tiny-usb` control transfers. If pins are not yet configured, it auto-claims default flexible pins (GP10/11) on first access or returns `-ENODEV` if in strict claim mode.

### 3. Dedicated Hardware Transceiver Pins
- CAN: GPIO 18 (`CAN_TX`), GPIO 19 (`CAN_RX`), GPIO 22 (`CAN_TERM`).
- RS-485: GPIO 12 (`UART0_TX`), GPIO 13 (`UART0_RX`), GPIO 14 (`RS485_DE`).
