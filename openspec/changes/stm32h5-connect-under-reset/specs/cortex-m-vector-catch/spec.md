## MODIFIED Requirements

### Requirement: Connect-under-reset attachment
The system SHALL support driving target physical reset (`nRST`) low during SWD initialization and holding it until vector catch is armed. On ARMv8-M dual-AP architectures (such as STM32H5), the system SHALL configure low-power debug clock gating via AP 0 (`DBGMCU_CR` at `0xE00E4004`) while under reset, arm CoreSight vector catch across reset release, and re-establish SWD communication over AP 1 (`AHB-AP`) with Non-Secure privileged CSW (`0x23000012`).

#### Scenario: Successful connect-under-reset on standard Cortex-M
- **WHEN** the probe connects to a standard Cortex-M target with `connect_under_reset` requested
- **THEN** the probe asserts `nRST` low, executes SWD line initialization sequence, writes `DEMCR.TRCENA = 1` and `DEMCR.VC_CORERESET = 1`, releases `nRST`, and halts the target at cycle 0 of the reset vector before executing user instructions

#### Scenario: Successful connect-under-reset on ARMv8-M STM32H5
- **WHEN** the probe connects to an ARMv8-M target (`DPIDR: 0x6BA02477` or `0x0BE12477`) under reset
- **THEN** the probe holds `nRST` low, powers up debug, writes `0x06` (`DBG_STOP | DBG_STANDBY`) to AP 0 DBGMCU (`0xE00E4004`), releases `nRST` high, halts the core at cycle 0, and switches to AP 1 (`AHB-AP`) with `CSW = 0x23000012`

#### Scenario: Connect-under-reset timeout
- **WHEN** the target fails to halt within 500ms after releasing `nRST`
- **THEN** the system returns `ERR TARGET RESET TIMEOUT` and releases pin ownership

## ADDED Requirements

### Requirement: Automatic connect-under-reset for flash programming
The system SHALL automatically employ connect-under-reset when initiating flash erase or programming on targets where runtime SWD attachment fails or on ARMv8-M targets requiring cycle-0 halting to prevent faulty user firmware from disabling debug pins.

#### Scenario: Flash programming with connect-under-reset
- **WHEN** `POST /api/v1/programmer/flash` or `POST /api/v1/programmer/erase` is called for an STM32H5 target
- **THEN** the programmer connects under reset to halt the core at instruction 0 before executing flash algorithm erase or programming pages, ensuring AHB bus clocks remain active and user firmware cannot disconnect SWD
