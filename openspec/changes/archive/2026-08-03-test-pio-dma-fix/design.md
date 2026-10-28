## Context

AgentLA is an RP2350-based multi-tool (logic analyzer + SWD + CMSIS-DAP) running TinyUSB CDC+HID composite. The SWD engine uses PIO1 with a modular origin-dispatch architecture: 7 sub-programs at fixed PIO instruction addresses, chained via `pull block` + `out pc, 32`. C code sends [origin, bit_count, data, ...] buffers to the TX FIFO via DMA.

Two bugs were identified and fixed in the PIO program (`agentla_swd.pio`):
1. Origin 0 lacked a `pull block` before `out pc, 32`, so the SM read the uninitialized OSR (0xFFFFFFFF) and jumped to garbage PC.
2. Each `out pc, 32` triggers autopull which consumes an extra TX FIFO word, causing FIFO starvation without a trailing dummy word.

The firmware is built (`build/AgentLA.uf2`) but completely untested. The CH32V203 target is connected via 2-wire RV-SWD (SWCLK=GPIO4, SWDIO=GPIO5, NRESET=GPIO6). Previous debugging was hampered by printf-based diagnostics that blocked the main loop and by `pio_sm_get_blocking()` calls that hung forever when the PIO didn't respond.

## Goals / Non-Goals

**Goals:**
- Verify PIO DMA transfers complete without hanging (swd_write_bits and swd_read_bits)
- Verify swd_line_reset → swd_dormant_to_swd → swd_read_idcode sequence succeeds
- Detect and diagnose any remaining PIO SM stalls (wrong PC, TX stall, RX empty)
- Provide a minimal wire-level loopback test (no target required) to isolate PIO vs. target issues

**Non-Goals:**
- Full CH32V203 flash erase/write testing (blocked on SWD working first)
- PIO program restructuring beyond the already-applied fix
- CMSIS-DAP or logic analyzer functionality testing

## Decisions

**D1: Debug via LED toggles, not printf**
Printf over CDC1 is non-blocking but the USB stack requires `tud_task()` to be called frequently. During a PIO DMA hang, the main loop is blocked and USB polling stops. LED toggles on a spare GPIO (e.g., GPIO25 = onboard LED) are atomic and visible on a scope.
→ Alternative: CDC1 printf — rejected because it silently drops output when the main loop stalls.

**D2: Add DMA timeout in tap_pio_dma_send_uint32**
Wrap `dma_channel_wait_for_finish_blocking` with a polling loop that checks `dma_channel_is_busy()` and a cycle counter. If DMA doesn't complete within 100ms, abort and return error. This prevents the entire firmware from locking up on a PIO hang.
→ Alternative: Reset PIO SM on timeout — rejected as too aggressive for first pass; log the stall state first.

**D3: PIO SM state dump after hang**
On DMA timeout, read and store: `pc` (via `pio_sm_get_pc`), `exec stalled` bit, `TX FIFO level`, `RX FIFO level`, `TXSTALL` debug bit, `SM state` register. Expose via a DIAG CLI command on CDC0.
→ Alternative: Just return error code — rejected because we need to diagnose *where* the SM got stuck.

**D4: Loopback test shorts SWDIO to SWCLK via external jumper**
A minimal test: configure SWCLK as output, SWDIO as input (or vice versa), toggle SWCLK and read SWDIO back. This verifies PIO can drive and read pins without needing a target. Requires a physical jumper wire between GPIO4 and GPIO5.
→ Alternative: Internal PIO loopback — not available on RP2350 PIO.

## Risks / Trade-offs

- **[Risk] DMA timeout value too short** → Set to 100ms (generous for PIO operations that complete in microseconds). Can be tuned.
- **[Risk] LED toggle overhead corrupts SWD timing** → Only toggle BEFORE and AFTER PIO operations, never during. No side-set conflict since LED GPIO is not in PIO pin range.
- **[Risk] PIO SM state dump reads stale values** → Read registers immediately on timeout, before any other PIO operation. Store in volatile globals.
- **[Trade-off] Loopback requires physical jumper** → Documented in programmer.md; skip if no jumper available, fall back to target-based tests only.

## Migration Plan

1. Flash firmware via BOOTSEL (drag-drop `build/AgentLA.uf2`)
2. Run CLI commands on CDC0: `DIAG` (PIO state), `LOOPBACK` (wire test), `SWDTEST` (full sequence)
3. If SWDTEST fails, run `DIAG` to inspect PIO state registers
4. If LOOPBACK fails, the PIO fix is still broken — inspect with logic analyzer on GPIO4/5
5. Once SWDTEST passes, proceed to CH32V203 probe/flash testing

## Open Questions

- Should the DMA timeout be configurable via CLI command or compile-time only?
- Is the onboard LED (GPIO25) available, or is it used by another function?
- What PIO instruction address does the SM actually land on when it hangs? (Needs DIAG output to answer)
