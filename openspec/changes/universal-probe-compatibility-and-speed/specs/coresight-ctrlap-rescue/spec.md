## ADDED Requirements

### Requirement: CoreSight multi-AP selection
The system SHALL support selecting non-zero CoreSight Access Ports (`APSEL`) via the DP SELECT register (`0x08`).

#### Scenario: Select Access Port 1
- **WHEN** the system communicates with a target debug module on Access Port 1
- **THEN** it writes `(1 << 24)` to DP SELECT and addresses registers on that AP

### Requirement: Nordic CTRL-AP mass erase
The system SHALL support unprotecting locked Nordic nRF52 and nRF53 microcontrollers by issuing an `ERASEALL` command to CoreSight CTRL-AP.

#### Scenario: Successful nRF chip unlock
- **WHEN** a locked Nordic target with `APPROTECT` enabled is targeted for rescue
- **THEN** the system addresses CTRL-AP (AP #1), writes `0x01` to `ERASEALL` (register `0x004`), polls `ERASEALLSTATUS` until clear, resets the target, and restores full AHB-AP debug access

#### Scenario: REST API nRF rescue
- **WHEN** an HTTP client sends `POST /api/v1/programmer/rescue/nrf`
- **THEN** the system executes the CTRL-AP mass-erase unlock sequence and returns `HTTP 200` with recovery status
