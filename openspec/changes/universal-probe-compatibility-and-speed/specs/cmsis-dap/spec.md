## ADDED Requirements

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

### Requirement: DAP_SWJ_Sequence command
The system SHALL support `DAP_SWJ_Sequence` (command `0x12`) to transmit arbitrary bit sequences for JTAG-to-SWD and dormant state switching.

#### Scenario: Switch line state sequence
- **WHEN** host sends `DAP_SWJ_Sequence` with bit count and sequence bytes
- **THEN** the system clocks the exact bit pattern onto SWDIO/SWCLK using the PIO state machine
