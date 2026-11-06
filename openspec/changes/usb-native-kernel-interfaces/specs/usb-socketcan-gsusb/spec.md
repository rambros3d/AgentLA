# usb-socketcan-gsusb Specification

## Purpose
Provide a native, driverless USB-to-CAN interface conforming to the Geschwister Schneider (`gs_usb`) protocol supported natively by the Linux kernel (`drivers/net/can/usb/gs_usb.c`).

## Requirements

### Requirement: Native SocketCAN gs_usb Interface
The firmware SHALL include a USB Vendor Class interface conforming to the `gs_usb` protocol with 1 Bulk IN and 1 Bulk OUT endpoint.

#### Scenario: Host enumerates SocketCAN interface
- **WHEN** AgentLA is connected to a host workstation
- **THEN** the USB descriptor exposes an interface with string "AgentLA SocketCAN gs_usb" and Bulk IN/OUT endpoints ready for `gs_usb` driver attachment.

### Requirement: CAN Control Transfer Handling
The firmware SHALL handle `gs_usb` control requests on Endpoint 0, including `HOST_FORMAT`, `BITTIMING`, `MODE`, `BT_CONST`, `DEVICE_CONFIG`, and termination controls.

#### Scenario: Host negotiates bit timing and starts interface
- **WHEN** host sends `GS_USB_BREQ_BT_CONST`
- **THEN** firmware returns standard bit timing parameters (fclk = 48 MHz).
- **WHEN** host sends `GS_USB_BREQ_MODE` with `GS_CAN_MODE_START`
- **THEN** CAN transceiver pins are claimed and driven into recessive state.

### Requirement: Standby and Pin Lifecycle
The system SHALL keep CAN transceiver pins tri-stated when `can0` is down (`GS_CAN_MODE_RESET`) and release pin ownership when idle.

#### Scenario: Bringing down can0
- **WHEN** host executes `ip link set can0 down` or sends `GS_CAN_MODE_RESET`
- **THEN** firmware releases the pin claims and sets GPIO 18, 19, and 22 into high-impedance mode.

### Requirement: Transmit Echo Loopback
The firmware SHALL echo transmitted frames with valid `echo_id` back on the Bulk IN endpoint to acknowledge transmission to the host SocketCAN network stack.
