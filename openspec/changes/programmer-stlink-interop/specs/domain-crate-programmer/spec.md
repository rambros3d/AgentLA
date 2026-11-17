## ADDED Requirements

### Requirement: ST-Link command reply contract

The ST-Link personality SHALL compose its command reply from protocol state alone
and SHALL NOT delay a reply on target access. Specifically, `DBG_APIV1_ENTER`
and `DBG_APIV2_ENTER` SHALL report `STLINK_OK` without first completing target
detection, and SHALL record the detected IDCODE for subsequent commands.

#### Scenario: Enter SWD with no target attached

- **WHEN** a host sends `DBG_APIV2_ENTER` with `DBG_ENTER_SWD` and no target is
  connected
- **THEN** the probe returns `STLINK_OK` within the host's command timeout
- **AND** the following `DBG_READCOREID` returns an IDCODE of 0

#### Scenario: Enter SWD with a sleeping target

- **WHEN** a host sends `DBG_APIV2_ENTER` for a target whose debug port is not
  yet powered up
- **THEN** the probe still returns `STLINK_OK` promptly
- **AND** the IDCODE read is attempted on a best-effort basis after the reply

### Requirement: Core debug commands select AP 0 with default CSW

The ST-Link personality SHALL select AP 0 with the default CSW before executing
any core debug command, except the DAP-level commands that manage their own AP
state (`RW_MISC_IN`, `RW_MISC_OUT`, `INIT_AP`, `CLOSE_AP_DBG`, `READ_DAP_REG`,
`WRITE_DAP_REG`).

#### Scenario: Halt after a memory access

- **WHEN** a memory command has selected AP 1 and a subsequent `DBG_RUNCORE` or
  `DBG_FORCEDEBUG` arrives
- **THEN** the probe selects AP 0 before touching `DHCSR`
- **AND** the halt or resume takes effect on the core

### Requirement: ST-Link transport errors are observable

The ST-Link USB task SHALL log failures to read a command from the OUT endpoint
and failures to write a reply to the IN endpoint. It SHALL NOT discard those
results silently.

#### Scenario: Reply write fails

- **WHEN** the IN endpoint rejects a reply write
- **THEN** the probe emits a warning containing the reply length and the error
- **AND** the command loop continues rather than terminating