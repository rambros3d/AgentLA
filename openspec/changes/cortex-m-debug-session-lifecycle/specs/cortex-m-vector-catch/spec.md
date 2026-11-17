## MODIFIED Requirements

### Requirement: Connect-under-reset attachment

The system SHALL support driving target physical reset (`nRST`) low during SWD initialization and
holding it until vector catch is armed. On ARMv8-M dual-AP architectures (such as STM32H5), the
system SHALL configure low-power debug clock gating via AP 0 (`DBGMCU_CR` at `0xE00E4004`) while
under reset, arm CoreSight vector catch across reset release, and re-establish SWD communication
over AP 1 (`AHB-AP`) with Non-Secure privileged CSW (`0x23000012`).

Before arming vector catch, the system SHALL snapshot the current `DEMCR` (`0xE000EDFC`) value so
that it can be restored when the debug session ends. A failed snapshot read SHALL NOT abort the
attach; it SHALL record that no snapshot exists.

#### Scenario: Successful connect-under-reset on standard Cortex-M

- **WHEN** the probe connects to a standard Cortex-M target with `connect_under_reset` requested
- **THEN** the probe asserts `nRST` low, executes SWD line initialization sequence, writes
  `DEMCR.TRCENA = 1` and `DEMCR.VC_CORERESET = 1`, releases `nRST`, and halts the target at cycle 0
  of the reset vector before executing user instructions

#### Scenario: Successful connect-under-reset on ARMv8-M STM32H5

- **WHEN** the probe connects to an ARMv8-M target (`DPIDR: 0x6BA02477` or `0x0BE12477`) under reset
- **THEN** the probe holds `nRST` low, powers up debug, writes `0x06` (`DBG_STOP | DBG_STANDBY`) to
  AP 0 DBGMCU (`0xE00E4004`), releases `nRST` high, halts the core at cycle 0, and switches to AP 1
  (`AHB-AP`) with `CSW = 0x23000012`

#### Scenario: DEMCR snapshot before arming

- **WHEN** the probe is about to arm `DEMCR.VC_CORERESET` on a target whose `DEMCR` currently reads
  `0x0100_0001`
- **THEN** the probe stores `0x0100_0001` as the session's prior debug configuration
- **AND** it writes `0x0100_0001` to arm vector catch without losing the snapshot

#### Scenario: Connect-under-reset timeout

- **WHEN** the target fails to halt within 500ms after releasing `nRST`
- **THEN** the system returns `ERR TARGET RESET TIMEOUT` and releases pin ownership

## ADDED Requirements

### Requirement: Debug session teardown

The system SHALL restore target debug state when an SWD debug session ends, regardless of whether
the operation succeeded, failed, or returned early. Teardown SHALL be best-effort: it SHALL NOT
perform unbounded polling and SHALL NOT change the result reported to the caller.

Teardown SHALL apply a detach policy:

- `Leave` - restore the snapshotted `DEMCR` only; do not modify `DHCSR`
- `Halt` - restore the snapshotted `DEMCR`, then force `DHCSR.C_HALT`
- `Run` - restore the snapshotted `DEMCR`, then clear `DHCSR.C_HALT` so user code executes

Restoring `DEMCR` SHALL clear `DEMCR.VC_CORERESET` unless the snapshotted value had it set, and
SHALL preserve `DEMCR.TRCENA` when it was set before the session began.

#### Scenario: Flash then run

- **WHEN** `POST /api/v1/programmer/flash` completes successfully on an ARMv8-M target that was
  attached under reset
- **THEN** teardown restores the prior `DEMCR`, clearing `VC_CORERESET`
- **AND** the core resumes and executes the freshly programmed application without re-entering
  vector catch on the next software system reset

#### Scenario: GDB client disconnect

- **WHEN** a GDB client disconnects from TCP port 3333
- **THEN** the probe restores the snapshotted `DEMCR` and resumes the core
- **AND** a subsequent `NVIC_SystemReset()` on the target is not intercepted by CoreSight

#### Scenario: Session ends with the core halted

- **WHEN** an endpoint that detaches with policy `Halt` finishes
- **THEN** `DHCSR.S_HALT` remains set and `DEMCR.VC_CORERESET` is cleared

#### Scenario: Teardown on an error path

- **WHEN** a flash operation fails verification and returns an error
- **THEN** the SWD session is torn down with the same policy as a successful run
- **AND** the target is not left with vector catch armed

#### Scenario: Teardown with an unreachable target

- **WHEN** the target stops responding before the session ends
- **THEN** teardown attempts the `DEMCR` restore once, ignores the failure, and releases pins
  without returning an error to the caller