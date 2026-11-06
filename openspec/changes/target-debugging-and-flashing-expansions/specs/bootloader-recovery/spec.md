## MODIFIED Requirements

### Requirement: Bootloader-based recovery
The system SHALL support flashing and recovering parts through vendor UART bootloaders when debug recovery is unavailable.

#### Scenario: ST part recovery via AN3155
- **WHEN** a locked ST part that exposes the AN3155 bootloader is placed in bootloader mode
- **THEN** the system performs 0x7F auto-baud sync, readout unprotect (0x92), flash mass-erase, and resets the target to application

#### Scenario: Bootloader unavailable
- **WHEN** the target does not respond to bootloader sync within 1000ms
- **THEN** the system returns `ERR BOOTLOADER NOT FOUND` and releases BOOT0 and UART pins

## ADDED Requirements

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
