## ADDED Requirements

### Requirement: Connect-under-reset attachment
The system SHALL support driving target physical reset (`nRST`) low during SWD initialization and holding it until vector catch is armed.

#### Scenario: Successful connect-under-reset
- **WHEN** the probe connects to a Cortex-M target with `connect_under_reset` requested
- **THEN** the probe asserts `nRST` low, executes SWD line initialization sequence, writes `DEMCR.TRCENA = 1` and `DEMCR.VC_CORERESET = 1`, releases `nRST`, and halts the target at cycle 0 of the reset vector before executing user instructions

#### Scenario: Connect-under-reset timeout
- **WHEN** the target fails to halt within 500ms after releasing `nRST`
- **THEN** the system returns `ERR TARGET RESET TIMEOUT` and releases pin ownership

### Requirement: Software reset with vector catch
The system SHALL support triggering a software system reset via `AIRCR` while keeping vector catch armed when physical `nRST` is unasserted or disconnected.

#### Scenario: Software system reset
- **WHEN** a software reset is requested on a connected Cortex-M target
- **THEN** the probe writes `0x05FA0004` to `AIRCR` (`SYSRESETREQ`), monitors `DHCSR.S_HALT`, and confirms the target halts immediately upon reset recovery

### Requirement: Reset and catch REST endpoint
The system SHALL expose a REST API endpoint to trigger connect-under-reset and vector catch attachment.

#### Scenario: REST API reset-catch trigger
- **WHEN** an HTTP client sends `POST /api/v1/programmer/reset-catch` with target port parameter
- **THEN** the probe executes the connect-under-reset sequence and returns `HTTP 200` with target status and initial Program Counter
