## Why

To achieve 100% reliable, zero-touch SWD connection and flash programming on STM32F103 (Blue Pill) targets, we need an autonomous diagnostic and recovery workflow. This proposal establishes automated SWD turnaround timing tuning, connect-under-reset vector catch, FPEC flash programming verification, and real-time logic analyzer signal validation without requiring user intervention.

## What Changes

- Add **Autonomous Multi-Phase Diagnostic Runner** (`test/test_stm32f1_autonomous_debug.py`) covering:
  - **Phase 1**: SWD Prescaler & Turnaround Cycle Sweep (50 kHz to 1000 kHz, 1-cycle vs 2-cycle turnaround).
  - **Phase 2**: Connect-Under-Reset Vector Catch (`nRESET` pulse on GPIO6 + `VC_CORERESET` in `DEMCR`).
  - **Phase 3**: FPEC Flash Controller Sector Programming & 100% readback byte verification.
  - **Phase 4**: Real-time logic analyzer sampling on `GPIO18` (connected to Blue Pill `PC13` LED pin).
- Add **SWD Turnaround Cycle Configuration** (`swd_set_turnaround_cycles`) in `firmware/src/programmer/swd.c` and CDC command `SWD TURNAROUND <1|2>`.
- Update `docs/programmer.md` with autonomous debugging routines and SWD turnaround tuning procedures.

## Capabilities

### New Capabilities
- `stm32f1-autonomous-debug`: Autonomous 4-phase diagnostic and recovery system to debug SWD connection states, auto-tune clock prescalers/turnaround timing, and verify all probe functions on STM32F1 (Blue Pill) hardware.

### Modified Capabilities
- None.

## Impact

- **Firmware**: Adds `swd_set_turnaround_cycles()` in `firmware/src/programmer/swd.c` and `SWD TURNAROUND` command in `firmware/src/main.c`.
- **Test Harness**: Adds `test/test_stm32f1_autonomous_debug.py`.
- **Documentation**: Updates `docs/programmer.md`.
