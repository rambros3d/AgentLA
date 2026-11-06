# cmsis-dap Specification

## Purpose
TBD - created by archiving change add-swd-debug-probe. Update Purpose after archive.
## Requirements
### Requirement: USB HID Device Enumeration
The system SHALL enumerate as a USB HID device with VID 0x1209 and PID 0x414D.

#### Scenario: Device enumeration
- **WHEN** AgentLA is connected to USB host
- **THEN** device appears as HID device with correct VID/PID

#### Scenario: HID report descriptor
- **WHEN** host queries HID report descriptor
- **THEN** system returns 64-byte input and 64-byte output reports

### Requirement: DAP_Info Command
The system SHALL respond to DAP_Info (0x00) with device capabilities.

#### Scenario: Query firmware version
- **WHEN** host sends DAP_Info with id=0x00
- **THEN** system returns firmware version string

#### Scenario: Query capabilities
- **WHEN** host sends DAP_Info with id=0xF0
- **THEN** system returns SWD support flag

### Requirement: DAP_Connect Command
The system SHALL initialize SWD connection to target on DAP_Connect.

#### Scenario: SWD connect
- **WHEN** host sends DAP_Connect with mode=1 (SWD)
- **THEN** system initializes PIO for SWD and returns success

#### Scenario: JTAG connect (unsupported)
- **WHEN** host sends DAP_Connect with mode=0 (JTAG)
- **THEN** system returns error (JTAG not supported)

### Requirement: DAP_Transfer Command
The system SHALL perform SWD read/write operations via DAP_Transfer.

#### Scenario: Read DP register
- **WHEN** host sends DAP_Transfer to read DPIDR
- **THEN** system returns 32-bit IDCODE value

#### Scenario: Write AP register
- **WHEN** host sends DAP_Transfer to write AP register
- **THEN** system performs SWD write and returns ACK status

### Requirement: DAP_SWJ_Clock Command
The system SHALL configure SWD clock frequency via DAP_SWJ_Clock.

#### Scenario: Set clock speed
- **WHEN** host sends DAP_SWJ_Clock with freq=1000000
- **THEN** system configures PIO for 1 MHz SWCLK

#### Scenario: Invalid clock speed
- **WHEN** host sends DAP_SWJ_Clock with freq > 4000000
- **THEN** system returns error status

### Requirement: DAP_SWJ_Sequence Command
The system SHALL send custom bit sequences via DAP_SWJ_Sequence.

#### Scenario: Line reset sequence
- **WHEN** host sends DAP_SWJ_Sequence with 56 high bits
- **THEN** system clocks out the reset sequence on SWDIO

#### Scenario: JTAG-to-SWD switch
- **WHEN** host sends DAP_SWJ_Sequence with 0xE79E pattern
- **THEN** system sends the JTAG-to-SWD switching sequence


### Requirement: CMSIS-DAP v2 WinUSB bulk interface
The system SHALL provide a CMSIS-DAP v2 USB Bulk interface using Microsoft OS 2.0 WinUSB descriptors to support driverless zero-polling-latency communication.

#### Scenario: WinUSB bulk enumeration
- **WHEN** AgentLA is connected to a host supporting CMSIS-DAP v2
- **THEN** the device exposes Bulk IN and Bulk OUT endpoints operating with 64-byte or 512-byte packets without HID polling intervals

#### Scenario: High-speed packet processing
- **WHEN** host sends a CMSIS-DAP v2 packet over the Bulk OUT endpoint
- **THEN** the firmware processes the DAP command and returns the response over Bulk IN within 50µs

### Requirement: DAP_TransferBlock command
The system SHALL support `DAP_TransferBlock` (command `0x06`) for contiguous block read and write operations over SWD.

#### Scenario: Contiguous block memory read
- **WHEN** host issues `DAP_TransferBlock` requesting $N$ words from target memory
- **THEN** the system executes SWD posted reads with auto-incrementing TAR and returns all $N$ data words in a single response packet

#### Scenario: Contiguous block memory write
- **WHEN** host issues `DAP_TransferBlock` writing $N$ words to target memory
- **THEN** the system streams the data words via SWD with auto-incrementing TAR and returns the transfer count and ACK status
