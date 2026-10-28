# programmer-uart Specification

## Purpose
TBD - created by archiving change programmer-interface. Update Purpose after archive.
## Requirements
### Requirement: UART flash programming for STM32 targets
The system SHALL support flashing STM32 MCUs via UART bootloader protocol (AN3155) using GPIO8 (TX) and GPIO9 (RX).

#### Scenario: STM32 sync and erase
- **WHEN** agent sends `PROG STM32 SYNC` command
- **THEN** firmware sends 0x7F sync byte at 115200 baud, receives ACK (0x79), and returns `OK STM32 SYNCED`

#### Scenario: STM32 write memory
- **WHEN** agent sends `PROG STM32 WRITE <addr> <len>` followed by firmware data
- **THEN** firmware erases target memory at address, writes data, verifies checksum, and returns `OK STM32 WRITE <len>`

#### Scenario: STM32 verify
- **WHEN** agent sends `PROG STM32 VERIFY <addr> <len>`
- **THEN** firmware reads target memory and returns `OK STM32 VERIFY PASS` or `ERR STM32 VERIFY FAIL`

### Requirement: UART flash programming for RP2350 targets
The system SHALL support flashing RP2350 MCUs via UART bootloader protocol using GPIO8 (TX) and GPIO9 (RX).

#### Scenario: RP2350 sync
- **WHEN** agent sends `PROG RP2350 SYNC` command
- **THEN** firmware sends sync sequence, receives ACK, and returns `OK RP2350 SYNCED`

#### Scenario: RP2350 binary load
- **WHEN** agent sends `PROG RP2350 LOAD <addr> <len>` followed by firmware data
- **THEN** firmware writes binary data to target address and returns `OK RP2350 LOAD <len>`

### Requirement: UART flash programming for NRF targets
The system SHALL support flashing NRF52 MCUs via UART DFU protocol using GPIO8 (TX) and GPIO9 (RX).

#### Scenario: NRF sync
- **WHEN** agent sends `PROG NRF SYNC` command
- **THEN** firmware initiates DFU session and returns `OK NRF SYNCED`

### Requirement: UART flash programming for CH32 targets
The system SHALL support flashing CH32 MCUs via WCH UART bootloader using GPIO8 (TX) and GPIO9 (RX).

#### Scenario: CH32 sync
- **WHEN** agent sends `PROG CH32 SYNC` command
- **THEN** firmware sends WCH sync sequence, receives ACK, and returns `OK CH32 SYNCED`

### Requirement: UART self-test loopback
The system SHALL support self-test verification of UART pins via loopback on GPIO8 and GPIO9.

#### Scenario: UART loopback test
- **WHEN** agent sends `PROG UART TEST` command
- **THEN** firmware connects GPIO8 to GPIO9 (via external loopback or internal mux), sends 0x55, verifies receive, and returns `OK UART TEST PASS` or `ERR UART TEST FAIL`

