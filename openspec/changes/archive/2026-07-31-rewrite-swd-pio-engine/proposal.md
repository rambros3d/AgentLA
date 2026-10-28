## Why

The current SWD engine (`agentla_swd.pio` + `agentla_swd.c`) fails with protocol error 5 (SWD_ERR_PROTOCOL) when connected to a Blue Pill target. Investigation revealed three root causes:

1. **Turnaround timing mismatch** — The PIO program hardcodes 4 NOP cycles for turnaround, but the C code assumes `turnaround_cycles = 1`. This causes ACK bits to be misaligned.
2. **Monolithic PIO program** — The single `get_next_cmd` dispatcher uses `out pc, 5` to jump to subroutines, making timing hard to control and debugging nearly impossible.
3. **Missing dormant-to-SWD sequence** — The ARM ADIv5 spec requires a dormant wake sequence that our code doesn't implement.

The MioLink project (references/MioLink) proves that a PIO-based SWD engine on RP2040 works correctly with BMP firmware. Their architecture uses 7 modular PIO sub-programs with fixed `.origin` addresses, DMA transfers, and hardware ACK validation.

## What Changes

Rewrite the SWD PIO engine from a monolithic command-dispatch architecture to MioLink's proven modular sub-program architecture:

- Replace the single `agentla_swd.pio` with 7 modular sub-programs using `.origin` directives
- Add DMA-based PIO FIFO transfers (send/receive)
- Add hardware ACK validation in PIO
- Fix turnaround timing by using a dedicated turnaround sub-program
- Add dormant-to-SWD and JTAG-to-SWD sequences from BMP's ADIV5 implementation
- Adapt for RP2350 (not RP2040) — different PIO hardware, higher clock speeds

## Capabilities

### New Capabilities
- Hardware-validated SWD ACK checking in PIO state machine
- Dormant-to-SWD wake sequence (ARM ADIv5 spec §5.3.4)
- DMA-based PIO FIFO transfers for zero-copy data movement
- Configurable SWD clock frequency via PIO clock divider

### Modified Capabilities
- SWD line reset timing updated to 60 HIGH cycles + 4 idle (from 56+2)
- SWD turnaround timing corrected to match PIO hardware (4 cycles)
- SWD transfer function rewritten to use PIO sub-program origins instead of command dispatch
- PIO program split into 7 modular sub-programs (from 1 monolithic program)

## Impact

- **Files modified:** `agentla_swd.pio`, `agentla_swd.c`, `agentla_swd.h`, `agentla_board_settings.h`
- **Files added:** `agentla_tap_pio.c`, `agentla_tap_pio.h` (PIO/DMA integration layer)
- **Files removed:** None (rewrite, not deletion)
- **Breaking changes:** SWD CLI commands remain the same, but internal implementation changes
- **Hardware compatibility:** Requires PIO1 on RP2350 (same pins GPIO4/GPIO5)
- **Tested with:** Blue Pill (STM32F103) via SWD
