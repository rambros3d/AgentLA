## Why

The SWD engine fails with protocol error 5 (`SWD_ERR_PROTOCOL`) when performing DP/AP writes to the Blue Pill target. Target detection (IDCODE read) appears to work, but any write operation (DP SELECT, AP CSW/TAR/DRW) fails. This blocks all flash programming functionality.

Root cause: Our `swd_transfer()` builds the PIO buffer differently from the proven MioLink reference implementation. Two critical structural differences cause the PIO state machine to get out of sync:

1. **Spurious `SWD_ORIGIN_START` appended** — We add an extra `out pc, 32` at the end of every transfer that consumes a TX FIFO entry and puts the PIO into an undefined state between transfers.
2. **Data+parity split into two `seq_out` calls** — MioLink packs data+parity+idle into a single `seq_out` with bit count `32+1+IDLE-1`, letting autopull seamlessly load the parity word mid-loop. We use two separate `seq_out` operations with an extra origin dispatch, adding 3 extra TX FIFO reads.

## What Changes

- Rewrite `swd_transfer()` in `agentla_swd.c` to build PIO buffers matching MioLink's `swdtap_adiv5_prepare_pio_seq()` pattern
- Combine data+parity+idle into a single `seq_out` operation (bit count = `32+1+IDLE-1`)
- Remove `SWD_ORIGIN_START` from write and read transfer paths
- Add configurable idle cycles (`TARGET_SWD_IDLE_CYCLES`) matching MioLink pattern
- Remove all debug `printf` statements added during investigation from `agentla_target.c` and `agentla_swd.c`
- Clean up redundant CSW/TAR writes in `agentla_target.c` flash algorithm
- Remove dead RP detection branch (`dpidr_identity == 0x6F`) from `agentla_target.c`

## Capabilities

### New Capabilities

_(none — this is a bug fix, not new functionality)_

### Modified Capabilities

- `swd-engine`: PIO buffer structure for SWD transfers rewritten to match MioLink reference. Combined data+parity output replaces split operations. Idle cycles added. SWD_ORIGIN_START removed from transfer endpoints.
- `flash-algorithms`: Debug printf statements removed. Redundant CSW/TAR writes eliminated. Dead RP detection branch removed.

## Impact

- **Files modified:** `firmware/src/agentla_swd.c` (SWD transfer rewrite), `firmware/src/agentla_target.c` (flash algorithm cleanup)
- **Breaking changes:** None — SWD CLI commands remain identical
- **Hardware compatibility:** RP2350 PIO1, GPIO4/GPIO5 (unchanged)
- **Tested with:** Blue Pill (STM32F103) via SWD
- **Reference:** MioLink `swdtap_adiv5_prepare_pio_seq()` in `references/MioLink/firmware/source/bmp/swdptap.c`
