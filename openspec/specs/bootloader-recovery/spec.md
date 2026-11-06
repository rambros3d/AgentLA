# bootloader-recovery

## Purpose
Supports flashing and recovering target microcontrollers through vendor UART bootloaders (such as ST AN3155 and WCH bootloaders) using dedicated BOOT0 and nRST hardware line orchestration when SWD/debug recovery is unavailable.

## Requirements

### Requirement: Bootloader-based recovery
The system SHALL support flashing and recovering parts through vendor UART bootloaders when debug recovery is unavailable.

#### Scenario: ST part recovery via AN3155
- **WHEN** a locked ST part that exposes the AN3155 bootloader is placed in bootloader mode
- **THEN** the system performs 0x7F auto-baud sync, readout unprotect (0x92), flash mass-erase, and resets the target to application

#### Scenario: Bootloader unavailable
- **WHEN** the target does not respond to bootloader sync within 1000ms
- **THEN** the system returns `ERR BOOTLOADER NOT FOUND` and releases BOOT0 and UART pins

### Requirement: ST AN3155 protocol implementation
The system SHALL implement the ST AN3155 serial bootloader protocol over target UART using 8E1 framing (8 data bits, even parity, 1 stop bit).

#### Scenario: Readout unprotect execution
- **WHEN** readout unprotect command is issued
- **THEN** the system transmits command byte 0x92 with checksum 0x6D, verifies ACK (0x79), waits for automatic hardware mass-erase completion, and receives final ACK (0x79)

#### Scenario: Target UART parity reconfiguration
- **WHEN** bootloader recovery is initiated
- **THEN** UART0 is automatically reconfigured from standard 8N1 to 8E1 mode for bootloader communication and restored to 8N1 on exit

### Requirement: Bootloader rescue REST API
The system SHALL expose a dedicated REST API endpoint to trigger STM32 bootloader recovery and unbricking.

#### Scenario: REST API rescue request
- **WHEN** an HTTP client sends `POST /api/v1/programmer/rescue/stm32`
- **THEN** the system initiates BOOT0 assertion, reset pulse, AN3155 sync, Readout Unprotect mass erase, and returns `HTTP 200` with recovery status

### Requirement: BOOT0 and reset orchestration
The system SHALL control BOOT0 (a dedicated GPIO selected in board configuration) and nRST (GPIO6) to enter and exit bootloader mode without external wiring beyond BOOT0.

#### Scenario: Enter bootloader mode
- **WHEN** bootloader recovery is requested
- **THEN** the system asserts BOOT0 high, pulses nRST low, waits, and then begins sync

#### Scenario: Exit bootloader mode
- **WHEN** bootloader recovery completes (success or failure)
- **THEN** the system releases BOOT0, pulses nRST, and restores normal operation

### Requirement: WCH bootloader support
The system SHALL support WCH bootloader-based flashing and recovery as a distinct vendor module.

#### Scenario: WCH sync
- **WHEN** a CH32 target exposes the WCH bootloader
- **THEN** the system performs WCH-specific sync, program, and verify flows

### Requirement: Bootloader documentation of coverage
The system SHALL document which target families and variants expose a bootloader suitable for recovery.

#### Scenario: Coverage documented
- **WHEN** a user consults the programming docs
- **THEN** the subset of ST/Nordic/WCH parts with recovery-reachable bootloaders is listed
