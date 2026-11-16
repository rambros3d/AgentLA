# stlink-probe-endpoint Specification

## Purpose
TBD - created by archiving change add-stlink-wchlink-programmer-modes. Update Purpose after archive.
## Requirements
### Requirement: ST-Link/V2-1 USB Interface Enumeration
The system SHALL enumerate as an ST-Link/V2-1 compatible USB device when ST-Link mode is active, presenting Vendor ID `0x0483`, Product ID `0x3752`, and a vendor debug interface with Bulk OUT (EP 0x01), Bulk IN (EP 0x81), and Trace IN (EP 0x82).

#### Scenario: ST-Link Device Enumeration
- **WHEN** ST-Link programmer mode is activated
- **THEN** the USB device presents VID `0x0483`, PID `0x3752`, and vendor debug interface on EP 0x01 (out), EP 0x81 (in), and EP 0x82 (trace in)

### Requirement: ST-Link Version Handshake
The system SHALL respond to the ST-Link `F1` version command reporting the dynamically configured version (default `V2J48M35` / `0x2C23`) to satisfy STM32CubeIDE, STM32CubeProgrammer, and OpenOCD compatibility checks across tool versions.

#### Scenario: Version query command
- **WHEN** the host issues command byte `0xF1` to the debug endpoint
- **THEN** the system returns a 6-byte version response matching the currently configured ST-Link version word

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

### Requirement: ST-Link SWO Trace Protocol Commands
The system SHALL handle ST-Link trace commands `DBG_APIV2_START_TRACE_RX` (`0xF2 0x40`), `DBG_APIV2_STOP_TRACE_RX` (`0xF2 0x41`), and `DBG_APIV2_GET_TRACE_NB` (`0xF2 0x42`), and stream exact trace packet byte counts over EP 0x82.

#### Scenario: Start and stop trace commands
- **WHEN** host issues `0xF2 0x40` specifying baud rate and buffer parameters
- **THEN** system starts the SWO receiver, configures the baud rate, and returns status OK

#### Scenario: Polled trace query and streaming
- **WHEN** host issues `0xF2 0x42` (GET_TRACE_NB)
- **THEN** system returns the count of available trace bytes and immediately sends that exact number of bytes on EP 0x82

### Requirement: ST-Link Target Voltage Reporting (Command 0xF7)
The system SHALL respond to command `0xF7` (`STLINK_GET_TARGET_VOLTAGE`) with an 8-byte response containing reference factor 2400 (bytes 0..3) and the live target voltage in millivolts (bytes 4..7).

#### Scenario: Query target voltage when target is powered
- **WHEN** host issues command byte `0xF7` and target rail is at 3300 mV
- **THEN** system responds with 8 bytes: `put32(resp[0..4], 2400)` and `put32(resp[4..8], 3300)`

#### Scenario: Query target voltage when target is unpowered
- **WHEN** host issues command byte `0xF7` and target rail is at 0 mV
- **THEN** system responds with 8 bytes: `put32(resp[0..4], 2400)` and `put32(resp[4..8], 0)`

### Requirement: Calibrated ST-Link SWD Frequency Configuration
The system SHALL handle command `DBG_APIV2_SWD_SET_FREQ` (`0xF2 0x43`) by mapping the 16-bit divisor parameter through the calibrated frequency mapping table and reconfiguring the hardware SWD clock divider.

#### Scenario: Set SWD clock frequency via ST-Link divisor command
- **WHEN** host sends `0xF2 0x43` with divisor 0
- **THEN** system reconfigures the SWD transport clock to 4000 kHz and returns status OK


