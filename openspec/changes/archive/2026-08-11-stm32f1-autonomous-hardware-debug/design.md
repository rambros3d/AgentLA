## Context

When connecting AgentLA to an STM32F103 (Blue Pill) target, the target SW-DP returns `raw_ack = 0x60000000` (`011` / `SWD_ACK_WAIT`). To establish a robust connection autonomously, the probe must perform automated prescaler sweeping, turnaround cycle adjustment, connect-under-reset vector catch, and end-to-end flash/signal verification.

## Goals / Non-Goals

**Goals:**
- Implement autonomous 4-phase diagnostic script (`test/test_stm32f1_autonomous_debug.py`) that sweeps clock prescalers (50 kHz to 1000 kHz), tests turnaround cycles, pulses `nRESET` with vector catch (`VC_CORERESET`), programs flash, and verifies PC13 LED logic analyzer output on `GPIO18`.
- Add `swd_set_turnaround_cycles(uint8_t cycles)` to `firmware/src/programmer/swd.c` and expose `SWD TURNAROUND <1|2>` over CDC serial in `main.c`.
- Automatically recover from `SWD_ACK_WAIT` without manual jumper or reset button presses.

**Non-Goals:**
- Rewriting underlying PIO state machine assembly instructions (`swd.pio`).

## Decisions

1. **Dynamic Turnaround Cycle Configuration**:
   - Allow switching `swd_state.turnaround_cycles` between 1 and 2 cycles at runtime to support non-standard SWJ-DP implementations.

2. **Connect-Under-Reset Vector Catch**:
   - Drive `nRESET` (`GPIO6`) LOW during line reset, set `VC_CORERESET` in `DEMCR` (`0xE000EDFC`), release `nRESET`, and halt CPU at reset vector before user code executes.

3. **Autonomous 4-Phase Diagnostic Suite**:
   - Construct `test_stm32f1_autonomous_debug.py` to systematically try combinations of SWD speed, turnaround cycles, and reset states until `DPIDR = 0x1BA01477` is latched.

## Risks / Trade-offs

- **[Risk]**: SWD lines floating due to disconnected ground or power.
  - **Mitigation**: Detect `ACK = 111 (NACK)` and output clear diagnostic instruction.
