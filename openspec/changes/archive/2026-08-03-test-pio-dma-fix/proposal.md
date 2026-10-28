## Why

The AgentLA RP2350 SWD engine has a critical PIO DMA hang bug that prevents all communication with CH32V203 targets. Two root causes were identified and fixed in code: (1) origin 0's `out pc, 32` reads garbage from the uninitialized OSR (0xFFFFFFFF) instead of pulling the first command origin from the TX FIFO, and (2) autopull at each `out pc, 32` consumes an extra word, starving the FIFO. The firmware is built but untested — we need a structured test plan to verify the fix works end-to-end and to debug any remaining issues.

## What Changes

- Add SWD debug diagnostics (LED toggles, not printf) to pinpoint exact hang location during DMA transfers
- Add timeout-based fallback in `tap_pio_dma_send_uint32` to detect and report DMA hangs instead of blocking forever
- Create a systematic test sequence: line reset → DPIDR read → CTRL/STAT write → target probe
- Add PIO state machine register dump (FIFO levels, PC, execution counter, stall bits) for post-hang forensic analysis
- Add GPIO toggle echo on SWDIO/SWCLK pins to verify PIO is actually driving the wire

## Capabilities

### New Capabilities
- `swd-dma-diagnostics`: DMA hang detection, PIO SM state dump, and GPIO-level signal verification for debugging PIO-SWD issues
- `swd-loopback-test`: Minimal SWD wire-level test that verifies PIO can toggle SWCLK/SWDIO and read back without a target

### Modified Capabilities
<!-- No existing spec-level behavior changes; the fix is to already-defined SWD behavior -->

## Impact

- **Code**: `agentla_tap_pio.c` (DMA timeout), `agentla_swd.c` (diagnostic hooks), `main.c` (test command)
- **PIO**: No further PIO program changes expected — the fix is already compiled
- **Hardware**: Requires CH32V203 connected via 2-wire RV-SWD (SWCLK=GPIO4, SWDIO=GPIO5)
- **USB**: Debug output goes to CDC1 (`/dev/ttyACM1`); CLI remains on CDC0 (`/dev/ttyACM0`)
- **Risk**: Diagnostic GPIO toggles may interfere with SWD timing if left enabled in production — must be compile-time gated
