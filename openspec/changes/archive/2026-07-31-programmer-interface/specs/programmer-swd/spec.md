## ADDED Requirements

### Requirement: SWD debug interface
The system SHALL provide SWD debug access using PIO1 on GPIO4 (SWCLK) and GPIO5 (SWDIO) for Phase 2 implementation.

#### Scenario: SWD connect
- **WHEN** agent sends `PROG SWD CONNECT` command
- **THEN** firmware initializes PIO1 for SWD protocol and returns `OK SWD CONNECTED`

#### Scenario: SWD read register
- **WHEN** agent sends `PROG SWD READ <addr>`
- **THEN** firmware performs SWD read cycle and returns `OK SWD READ <value>`

#### Scenario: SWD write register
- **WHEN** agent sends `PROG SWD WRITE <addr> <value>`
- **THEN** firmware performs SWD write cycle and returns `OK SWD WRITE`

### Requirement: SWD target reset
The system SHALL support SWD target reset via nRST pin (GPIO6).

#### Scenario: SWD target reset
- **WHEN** agent sends `PROG SWD RESET` command
- **THEN** firmware asserts nRST low for 100ms, releases, and returns `OK SWD RESET`
