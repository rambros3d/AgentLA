# programmer-mode Specification

## Purpose
TBD - created by archiving change add-swd-debug-probe. Update Purpose after archive.
## Requirements
### Requirement: SWD Mode Activation
The system SHALL support entering SWD debug mode via command.

#### Scenario: Enter SWD mode
- **WHEN** user sends "MODE SWD ON" command
- **THEN** system initializes PIO for SWD and responds "OK SWD MODE ON"

#### Scenario: Exit SWD mode
- **WHEN** user sends "MODE SWD OFF" command
- **THEN** system releases SWD pins and responds "OK SWD MODE OFF"

#### Scenario: SWD mode status
- **WHEN** user sends "MODE SWD STATUS" command
- **THEN** system responds with current SWD mode state

### Requirement: SWD Commands
The system SHALL support SWD-specific commands for manual testing.

#### Scenario: SWD IDCODE command
- **WHEN** user sends "SWD IDCODE" command
- **THEN** system reads and returns target IDCODE in hex format

#### Scenario: SWD READ command
- **WHEN** user sends "SWD READ DP 0x00" command
- **THEN** system reads DP register and returns value

#### Scenario: SWD WRITE command
- **WHEN** user sends "SWD WRITE AP 0x00 0x12345678" command
- **THEN** system writes value to AP register

#### Scenario: SWD RESET command
- **WHEN** user sends "SWD RESET" command
- **THEN** system performs line reset sequence

### Requirement: UART Bootloader Fallback
The system SHALL maintain UART bootloader as fallback option.

#### Scenario: UART bootloader still available
- **WHEN** SWD mode is active
- **THEN** UART bootloader commands remain functional

#### Scenario: Mode conflict detection
- **WHEN** user tries to enter both SWD and UART bootloader mode
- **THEN** system returns error (modes conflict on shared pins)

### Requirement: Pin Sharing Management
The system SHALL manage pin sharing between SWD and UART modes.

#### Scenario: SWD pin allocation
- **WHEN** SWD mode is activated
- **THEN** GPIO 6-7 are allocated to PIO1 for SWD

#### Scenario: UART pin allocation
- **WHEN** UART bootloader mode is activated
- **THEN** GPIO 4-5 are allocated to UART1 for bootloader

