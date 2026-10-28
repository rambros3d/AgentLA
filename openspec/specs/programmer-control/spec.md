# programmer-control Specification

## Purpose
TBD - created by archiving change programmer-interface. Update Purpose after archive.
## Requirements
### Requirement: Programmer mode activation
The system SHALL support exclusive programmer mode that reconfigures GPIO0-9 for programming operations.

#### Scenario: Enter programmer mode
- **WHEN** agent sends `MODE PROG ON` command
- **THEN** firmware reconfigures GPIO0-9 as programmer pins (UART TX/RX, I2C SDA/SCL, nRST, SWCLK, SWDIO, power enable) and returns `OK PROG MODE ON`

#### Scenario: Exit programmer mode
- **WHEN** agent sends `MODE PROG OFF` command
- **THEN** firmware releases GPIO0-9 from programmer mode and returns `OK PROG MODE OFF`

#### Scenario: Program mode status
- **WHEN** agent sends `MODE PROG STATUS` command
- **THEN** firmware returns `OK PROG MODE <ON|OFF>` indicating current state

### Requirement: Target power control
The system SHALL control target MCU power via GPIO7 enable pin.

#### Scenario: Power on target
- **WHEN** agent sends `PROG POWER ON` command
- **THEN** firmware sets GPIO7 high, waits 100ms for regulator stabilization, and returns `OK POWER ON`

#### Scenario: Power off target
- **WHEN** agent sends `PROG POWER OFF` command
- **THEN** firmware sets GPIO7 low and returns `OK POWER OFF`

#### Scenario: Power status
- **WHEN** agent sends `PROG POWER STATUS` command
- **THEN** firmware reads GPIO7 state and returns `OK POWER <ON|OFF>`

### Requirement: Target hardware reset
The system SHALL control target nRST pin via GPIO6.

#### Scenario: Assert reset
- **WHEN** agent sends `PROG NRESET LOW` command
- **THEN** firmware sets GPIO6 low (open-drain) and returns `OK NRESET LOW`

#### Scenario: Release reset
- **WHEN** agent sends `PROG NRESET HIGH` command
- **THEN** firmware releases GPIO6 (floating, pulled up by external resistor) and returns `OK NRESET HIGH`

#### Scenario: Pulse reset
- **WHEN** agent sends `PROG NRESET PULSE` command
- **THEN** firmware asserts GPIO6 low for 100ms, releases, and returns `OK NRESET PULSE`

### Requirement: Programmer pin configuration query
The system SHALL report current programmer pin assignments.

#### Scenario: Query pin config
- **WHEN** agent sends `PROG PINS` command
- **THEN** firmware returns `OK PINS TX=8 RX=9 SDA=0 SCL=1 NRESET=6 SWCLK=4 SWDIO=5 POWER=7`

