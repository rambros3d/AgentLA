# usb-socketcan-gsusb Specification

## Purpose
TBD - created by archiving change usb-native-adapter-endpoints. Update Purpose after archive.
## Requirements
### Requirement: Native SocketCAN gs_usb Interface
The firmware SHALL include a USB Vendor Class interface conforming to the `gs_usb` protocol in the default USB configuration descriptor.

#### Scenario: Host enumerates SocketCAN interface
- **WHEN** AgentLA is connected to a Linux host
- **THEN** the host kernel loads `gs_usb.ko` and creates network interface `can0` in the `DOWN` state without requiring proprietary drivers

### Requirement: CAN Standby and Transmission Lifecycle
The system SHALL keep CAN transceiver pins tri-stated when `can0` is down and activate the transceiver when `can0` is brought up.

#### Scenario: Bringing up can0
- **WHEN** host executes `ip link set can0 up type can bitrate 500000`
- **THEN** system claims the CAN transceiver pins and enables the PIO-CAN controller

