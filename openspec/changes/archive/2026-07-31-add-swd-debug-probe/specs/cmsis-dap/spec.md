## ADDED Requirements

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
