## ADDED Requirements

### Requirement: RISC-V Debug Module Interface (DMI) Access
The system SHALL read and write DMI registers over 2-wire SWD and 1-wire SDI transports.

#### Scenario: DMCONTROL register access
- **WHEN** core halt or resume is requested
- **THEN** system writes `DMCONTROL` (`0x10`) register to control CPU execution state

#### Scenario: Abstract command execution
- **WHEN** CPU register or memory read is issued
- **THEN** system executes RISC-V abstract commands via `COMMAND` (`0x17`) and `DATA0` (`0x04`)
