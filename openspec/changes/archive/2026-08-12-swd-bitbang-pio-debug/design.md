## Context

The target Blue Pill board is wired with verified hardware connections, but the PIO SWD engine returns `raw_ack = 0xE0000000` (`111` NACK). To isolate whether the problem is in hardware line setup versus PIO state machine execution, we will build a software bitbanging SWD driver (`swd_bitbang_read_idcode()`), expose `SWD BITBANG` over CDC serial, run empirical diagnostics, and use those findings to fix the PIO state machine.

## Goals / Non-Goals

**Goals:**
- Implement software bitbanging SWD driver `swd_bitbang_read_idcode()` in `firmware/src/programmer/swd.c`.
- Expose `SWD BITBANG` CDC command in `firmware/src/main.c`.
- Isolate whether Bitbanging receives `ACK = 001 (OK)` and reads `DPIDR = 0x1BA01477`.
- Apply timing and pin direction fixes to `firmware/pio/swd.pio`.

**Non-Goals:**
- Replacing PIO permanently — bitbanging is a diagnostic tool to debug and perfect the PIO engine.

## Decisions

1. **Direct GPIO Bitbanging**:
   - Deinitialize PIO SM, claim GPIO4 (SWCLK) and GPIO5 (SWDIO) as standard GPIOs, and execute SWD line reset, JTAG-to-SWD select sequence (`0xE79E`), request (`0xA5`), turnaround, ACK read, and DPIDR read in pure C.

2. **Empirical Diagnostics**:
   - Run `SWD BITBANG` over CDC serial to determine working ACK status and IDCODE.
   - Use `SWD CAPTURE` logic analyzer to record both Bitbanging and PIO waveforms for exact side-by-side comparison.

## Risks / Trade-offs

- **[Risk]**: Software bitbanging speed is slower than PIO.
  - **Mitigation**: Use `sleep_us(5)` for ~100 kHz bitbanging, which is well within STM32F103 SWD timing specs.
