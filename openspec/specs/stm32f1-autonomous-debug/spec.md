# stm32f1-autonomous-debug Specification

## Purpose
TBD - created by archiving change stm32f1-autonomous-hardware-debug. Update Purpose after archive.
## Requirements
### Requirement: Autonomous 4-Phase Diagnostic Harness
The system SHALL provide an autonomous Python diagnostic script `test/test_stm32f1_autonomous_debug.py` that executes a 4-phase connection recovery, prescaler sweep, connect-under-reset, flash programming, and logic analyzer signal verification.

#### Scenario: Autonomous 4-phase execution
- **WHEN** the user runs `python3 test/test_stm32f1_autonomous_debug.py /dev/ttyACM0`
- **THEN** the script automatically sweeps prescalers, configures turnaround cycles, pulses reset, programs flash, and verifies PC13 LED output on `GPIO18` with zero manual intervention.

### Requirement: Configurable SWD Turnaround Cycles
The system firmware SHALL support setting the SWD turnaround cycles to 1 or 2 via `swd_set_turnaround_cycles()` and CDC serial command `SWD TURNAROUND <1|2>`.

#### Scenario: Set 2-cycle turnaround
- **WHEN** the user sends `SWD TURNAROUND 2\r\n` over CDC serial
- **THEN** the probe returns `OK SWD TURNAROUND 2 cycles` and updates `swd_state.turnaround_cycles` to 2.

