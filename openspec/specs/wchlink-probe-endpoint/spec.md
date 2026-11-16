# wchlink-probe-endpoint Specification

## Purpose
TBD - created by archiving change add-stlink-wchlink-programmer-modes. Update Purpose after archive.
## Requirements
### Requirement: WCH-Link USB Interface Enumeration
The system SHALL enumerate as a WCH-Link compatible USB device when WCH-Link mode is active, presenting Vendor ID `0x1a86`, Product ID `0x8010`, Product String `"WCH-Link"`, Command EP 0x01/0x81, and Data EP 0x02/0x82.

#### Scenario: WCH-Link Device Enumeration
- **WHEN** WCH-Link programmer mode is activated
- **THEN** the USB device presents VID `0x1a86`, PID `0x8010`, Product string `"WCH-Link"`, and endpoints 0x01/0x81 (command) and 0x02/0x82 (data)

### Requirement: WCH-Link Protocol Handshake
The system SHALL respond to WCH version query (`0x0D 0x01`), attach (`0x0D 0x02`), and speed configuration (`0x0C`) commands following firmware version 2.12 semantics.

#### Scenario: Query firmware version
- **WHEN** the host issues request `0x81 0x0D 0x01 0x01` on EP 0x01
- **THEN** the system replies with `0x82 0x0D 0x04 0x02 0x0C 0x01 0x00` on EP 0x81

#### Scenario: Attach target command
- **WHEN** the host issues attach request `0x81 0x0D 0x01 0x02` on EP 0x01
- **THEN** the system wakes the target over RVSWD, reads chip ID register 0x7F, and returns the 4-byte chip ID

### Requirement: 2-Wire RVSWD Protocol Framing
The system SHALL execute the 2-wire RVSWD half-duplex synchronous wire protocol with start/stop conditions, double-even-parity on headers and payloads, 4-bit zero padding, and validation of the target ACK nibble (`0111`).

#### Scenario: RVSWD DMI Read Operation
- **WHEN** a DMI read transfer is requested
- **THEN** the system clocks out header, doubled even parity, 4 padding zeros, samples 32-bit data, verifies payload parity, checks status nibble `0111`, and issues stop

#### Scenario: RVSWD DMI Write Operation
- **WHEN** a DMI write transfer is requested
- **THEN** the system clocks out header, doubled even parity, 4 padding zeros, 32-bit data, doubled payload even parity, samples target status nibble `0111`, and issues stop

### Requirement: Zero-Blob Flash Programming via Program Buffer
The system SHALL program target flash memory on CH32V targets using RISC-V 0.13 Debug Module abstract commands with `postexec` on `DM_PROGBUF0`, polling `WRBSY` in hardware.

#### Scenario: Fast flash page programming
- **WHEN** host streams flash page data over EP 0x02 following write command `0x02 0x02`
- **THEN** the system streams words into the target flash controller via the program buffer loop and returns acknowledgment `0x41 0x01 0x01 0x04` on EP 0x82

### Requirement: Dynamic WCH-Link Speed Tier Configuration
The system SHALL handle command `0x0C` by translating the speed tier byte into target clock frequency and reconfiguring the RVSWD PIO clock divider.

#### Scenario: Set RVSWD clock speed to 6 MHz
- **WHEN** host sends command `0x0C` with speed code 1
- **THEN** system configures RVSWD clock to 6000 kHz and returns status 0x01


