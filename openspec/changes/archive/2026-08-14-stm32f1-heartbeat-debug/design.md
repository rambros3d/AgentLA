## Context

To thoroughly validate AgentLA's Cortex-M hardware debugging capabilities on the Blue Pill (STM32F103), we need to test:
1. Flashing a non-trivial double-pulse heartbeat firmware.
2. Exercising low-level SWD debug registers (DHCSR, DEMCR, DCRSR, DCRDR).
3. Live CPU halt, single-stepping, core register inspection (R0–R15, xPSR), and memory inspection/mutation.
4. Logic analyzer PC13 LED output validation.

## Goals / Non-Goals

**Goals:**
- Update `test/bluepill_blink/src/main.cpp` to output a 1 Hz heartbeat double-beat pulse (100ms ON / 100ms OFF / 100ms ON / 700ms OFF).
- Add CDC serial command `DEBUG` subcommands (`HALT`, `RUN`, `STEP`, `REGS`, `READ <addr>`, `WRITE <addr> <val>`) in `main.c`.
- Build automated 5-stage python test script `test/test_stm32f1_heartbeat_debug.py`.

**Non-Goals:**
- Full GDB server implementation (gdb-remote protocol) — CDC CLI commands provide direct scriptable debug control.

## Decisions

1. **Direct SWD Debug Register Control**:
   - Use `DHCSR` (`0xE000EDF0`) for CPU Halt (`C_DEBUGEN | C_HALT = 0xA05F0003`) and Unhalt (`0xA05F0000`).
   - Use `DCRSR` (`0xE000EDF4`) and `DCRDR` (`0xE000EDF8`) for CPU core register access (R0–R15).
   - Use `DHCSR` bit 3 (`C_MASKINTS`) and bit 2 (`C_STEP`) for hardware single-stepping.

2. **Automated 5-Stage Verification**:
   - Stage 1: Build PlatformIO heartbeat firmware binary.
   - Stage 2: Program Flash via SWD FPEC controller.
   - Stage 3: Test CPU Halt, Single-Step, Core Register Read, and Memory Mutate.
   - Stage 4: Unhalt CPU and verify PC13 LED signal on `GPIO22` (L_CH2) using logic analyzer.

## Risks / Trade-offs

- **[Risk]**: Stepping while interrupts are active could jump into ISRs.
  - **Mitigation**: Set `C_MASKINTS` in `DHCSR` during single-step operations.

---

## Session State & Handoff (2026-08-13)

Firmware (`DEBUG HALT/RUN/STEP/REGS/READ/WRITE`) and stages 1–3 of the harness are
**verified on hardware**. Task 3.5 remains open, blocked by two issues — details and the
reset-gap root cause are recorded in `tasks.md` section 4.

- **Reset gap (root cause of stage 4/5)**: the test flashes without resetting the target, so
  the CPU faults (flash erased under running code; PC=0xFFFFFFFE lockup). A new firmware
  `DEBUG RESET` command (nRST GPIO6 pulse + SWD re-connect) was added but does not yet
  reliably reboot the CPU — nRST wiring/effectiveness unverified.
- **Harness fixes applied**: `send_cmd()` early-exit + idle grace (was always waiting the
  full 5 s timeout), `DEBUG REGS` OK-first ordering handled, memory-mutation parser now
  reads the value (was parsing the address).
- **New blocker**: LA `CAPTURE` hangs after `CONFIGURE channels=22` on the current build
  (`CAPTURING...` then stuck in `IsCapturing()`). Isolate before re-running stage 4/5.
