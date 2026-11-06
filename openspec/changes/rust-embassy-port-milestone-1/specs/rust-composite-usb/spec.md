## ADDED Requirements

### Requirement: Composite USB Device Enumeration
The firmware SHALL enumerate over USB 2.0 Full-Speed as a composite device presenting Dual CDC-ACM, CMSIS-DAP v1/v2, and CDC-NCM network interfaces.

#### Scenario: Host enumeration
- **WHEN** the probe is connected to a host via USB
- **THEN** the device enumerates with VID/PID, product string "AgentLA RP2350 Probe", and exposes the expected composite interfaces without host driver errors

### Requirement: Dual CDC-ACM Serial Channels
The firmware SHALL provide two independent CDC-ACM serial channels.

#### Scenario: CLI REPL access on CDC 0
- **WHEN** a serial terminal opens CDC Port 0 (`/dev/ttyACM0`)
- **THEN** the interactive AgentLA command prompt is presented and responds to commands

#### Scenario: Target UART bridge on CDC 1
- **WHEN** data is sent or received on CDC Port 1 (`/dev/ttyACM1`)
- **THEN** bytes are routed asynchronously to/from the target UART pins

### Requirement: CMSIS-DAP Scaffolding
The firmware SHALL provide CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (Bulk) endpoints.

#### Scenario: CMSIS-DAP DAP_Info query
- **WHEN** a debug tool (such as PyOCD or probe-rs) sends a `DAP_Info` packet to the HID or Bulk endpoint
- **THEN** the firmware replies with the probe hardware details, CMSIS-DAP version string, and capabilities
