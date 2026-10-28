## Context

The SWD engine uses PIO1 on RP2350 with 7 modular sub-programs at fixed `.origin` addresses. The C layer (`swd_transfer()`) builds a PIO command buffer and sends it via DMA, then reads the ACK from the RX FIFO.

The current implementation diverges from the proven MioLink reference in how PIO buffers are structured for data transfers. MioLink's `swdtap_adiv5_prepare_pio_seq()` is the gold standard — it has been validated on RP2040 hardware with BMP firmware.

## Root Cause Analysis

### Primary Issue: Register Address Mapping (FIXED)

The original code used `reg & 0x0C` to extract A[2:3] for `swd_build_request()`. This was wrong because:
- `reg & 0x0C` keeps bits [3:2] in place (e.g., reg=0x08 → 0x08)
- Then `a23 & 3` inside `swd_build_request` zeros them out again (0x08 & 3 = 0)
- Result: ALL register accesses mapped to offset 0x00

Fix: Changed to `(reg >> 2) & 3` which correctly extracts A[2:3] and shifts to [1:0].

### Secondary Issue: READ Path Buffer Structure (FIXED)

The original READ path appended `SWD_ORIGIN_START` (origin 0) after `TURNAROUND_TO_DRIVE`:
```
[7] SWD_ORIGIN_TURNAROUND_TO_DRIVE  → changes direction
[8] SWD_ORIGIN_START                → out pc, 32 → TX FIFO empty → STALL
```

This caused the PIO to stall at `out pc, 32` waiting for TX FIFO data that never arrives. While the PIO stalls harmlessly, it doesn't output idle cycles after reads.

Fix: Replaced with MioLink's pattern — `seq_out_turnaround` + idle count + zero data:
```
[7] SWD_ORIGIN_SEQ_OUT_TURNAROUND  → changes direction to drive
[8] idle_cycles - 1                 → loop counter for idle bits
[9] 0                               → idle data (SWDIO low)
```

The `seq_out` program shifts out the idle count bits, outputting 0s on SWDIO for the required idle cycles.

### Tertiary Issue: Insufficient Idle Cycles (FIXED)

Original: `idle_cycles = 2`
MioLink: `TARGET_SWD_IDLE_CYCLES = 8` (with static_assert >= 8)

Fix: Updated to 8 idle cycles to match MioLink.

## Goals / Non-Goals

**Goals:**
- Fix SWD DP/AP writes so flash programming works on Blue Pill (STM32F103)
- Match MioLink's PIO buffer structure exactly
- Remove all debug printf statements
- Clean up redundant code in flash algorithm

**Non-Goals:**
- Rewriting the PIO program (it's correct — only the C buffer builder is wrong)
- Changing pin assignments or PIO hardware configuration
- Adding new SWD features (this is a bug fix)

## Decisions

### Decision 1: Combine data+parity into single seq_out

**Choice:** Pack data (32 bits) + parity (1 bit) + idle cycles into one `seq_out` operation with bit count = `32 + 1 + IDLE - 1`.

**Why:** MioLink does this. The `seq_out` program loads bit count from TX FIFO, then shifts out bits. With autopull=ON, when the OSR empties after 32 data bits, autopull seamlessly loads the parity word from TX FIFO. The 33rd `out pins, 1` outputs the parity bit. This avoids an extra `out pc, 32` dispatch between data and parity.

### Decision 2: Remove SWD_ORIGIN_START from transfer paths

**Choice:** End transfers with the last `seq_out` or `seq_in` program's `out pc, 32`, which blocks waiting for the next command. Don't append `SWD_ORIGIN_START`.

**Why:** MioLink doesn't append an extra origin. The PIO naturally blocks at `out pc, 32` when the TX FIFO is empty. The CPU reads the ACK via `pio_sm_get_blocking`, then calls `pio_sm_clear_fifos` to reset for the next transfer.

### Decision 3: Add idle cycles to READ transfers

**Choice:** Include idle cycles in READ path via `seq_out_turnaround` + idle count + zero data.

**Why:** SWD spec requires idle cycles after writes. MioLink includes them in both read and write paths. Our original READ path had no idle cycles.

### Decision 4: Keep autopush=ON, threshold=32

**Choice:** Don't change the PIO SM config. Both MioLink and AgentLA use the same config.

**Why:** The `push` instruction works correctly with autopush on RP2040/RP2350 — it pushes the current ISR content to RX FIFO regardless of the threshold. The PIO program is correct; only the C buffer builder was wrong.

## Risks / Trade-offs

- **Risk:** Changing the PIO buffer structure could break other code paths. **Mitigation:** The function signature doesn't change. Only the internal buffer construction changes.

- **Risk:** Removing debug printfs might hide future issues. **Mitigation:** The printfs were added for investigation only. They should be removed before shipping.

- **Trade-off:** Matching MioLink's buffer structure exactly means our code looks more like a copy. **Benefit:** It's proven to work on hardware. Divergence caused the current bug.

## Quaternary Issue: Write Path Data+Parity FIFO Packing (FIXED)

The original write path sent data+parity as separate FIFO words but with a combined bit count (`32 + 1 + idle - 1`). This caused the `seq_out` PIO program to malfunction:

1. `out x, 32` loads bit count (40) from OSR
2. Loop shifts out 32 data bits → autopull loads parity word
3. Loop shifts out 1 parity bit → autopull loads idle data (0)
4. Loop shifts out 7 idle bits (OSR has 1 bit of idle data left)
5. `pull block` — **no-op** because autopull already loaded OSR
6. `out pc, 32` — tries to shift 32 bits, but only 1 bit remains in OSR
7. Autopull loads SWD_ORIGIN_SEQ_OUT, but the 31 stale bits corrupt the PC value

**Fix:** Send data (32 bits) and parity (1 bit) as **separate** `seq_out` operations:
- First `seq_out`: bit count = 31, data = val (32 data bits)
- Second `seq_out`: bit count = idle, data = parity (1 parity bit + idle cycles)

This matches MioLink's pattern where each `seq_out` handles a clean, self-contained data word.
