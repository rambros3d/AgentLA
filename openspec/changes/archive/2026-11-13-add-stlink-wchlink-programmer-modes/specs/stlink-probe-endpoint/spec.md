## ADDED Requirements

### Requirement: ST-Link/V2-1 USB Interface Enumeration
The system SHALL enumerate as an ST-Link/V2-1 compatible USB device when ST-Link mode is active, presenting Vendor ID `0x0483`, Product ID `0x3752`, and a vendor debug interface with Bulk OUT (EP 0x01) and Bulk IN (EP 0x81).

#### Scenario: ST-Link Device Enumeration
- **WHEN** ST-Link programmer mode is activated
- **THEN** the USB device presents VID `0x0483`, PID `0x3752`, and vendor debug interface on EP 0x01/0x81

### Requirement: ST-Link Version Handshake
The system SHALL respond to the ST-Link `F1` version command reporting ST-Link version `V2J48M35` (`0x2C23`) to satisfy STM32CubeIDE, STM32CubeProgrammer, and OpenOCD compatibility checks.

#### Scenario: Version query command
- **WHEN** the host issues command byte `0xF1` to the debug endpoint
- **THEN** the system returns a 6-byte version response matching `V2J48M35`

### Requirement: ST-Link Mode and Exit Commands
The system SHALL respond to ST-Link mode query (`F5`), DFU exit (`F3 07`), and debug mode entry (`F2 20`) commands.

#### Scenario: Query mode command
- **WHEN** the host issues command byte `0xF5`
- **THEN** the system responds with status `STLINK_MODE_DEBUG`

#### Scenario: Enter debug mode command
- **WHEN** the host issues command `0xF2 0x20`
- **THEN** the system initializes SWD line reset and returns the target CoreSight DP IDCODE

### Requirement: ST-Link Memory and Register Access
The system SHALL execute 8-bit, 16-bit, and 32-bit memory reads and writes, as well as core register access (`DCRSR`/`DCRDR`), via ST-Link `F2` command sequences over the SWD transport.

#### Scenario: 32-bit memory read
- **WHEN** the host issues `0xF2 0x32` with a valid 32-bit target address and byte count
- **THEN** the system reads data from target MEM-AP DRW with auto-increment and streams bytes to EP 0x81

#### Scenario: 32-bit memory write
- **WHEN** the host issues `0xF2 0x33` with target address, byte count, and data payload
- **THEN** the system writes data to target MEM-AP DRW and returns status OK

#### Scenario: Core register read
- **WHEN** the host issues `0xF2 0x3A` specifying a core register index
- **THEN** the system reads the register via DCRSR/DCRDR and returns the 32-bit value
