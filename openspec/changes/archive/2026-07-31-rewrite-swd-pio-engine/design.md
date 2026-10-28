## Context

The current SWD engine uses a monolithic PIO program with a command-dispatch architecture that has three critical bugs:
1. Turnaround timing hardcoded in PIO (4 NOPs) but C code assumes 1 cycle
2. Single `get_next_cmd` dispatcher with `out pc, 5` makes debugging impossible
3. No dormant-to-SWD wake sequence

The MioLink project (references/MioLink) proved that a PIO-based SWD engine works on RP2040 with BMP firmware. Their architecture uses 7 modular sub-programs with `.origin` directives, DMA transfers, and hardware ACK validation.

Our hardware: RP2350 with PIO1 (GPIO4=SWCLK, GPIO5=SWDIO). PIO1 on RP2350 has the same instruction set as RP2040 PIO but different base addresses and DMA capabilities.

## Goals / Non-Goals

**Goals:**
- Fix SWD protocol error 5 by correcting turnaround timing
- Adopt MioLink's modular PIO sub-program architecture
- Add hardware ACK validation in PIO
- Add dormant-to-SWD and JTAG-to-SWD sequences
- Maintain backward compatibility with existing SWD CLI commands
- Keep CMSIS-DAP HID and CDC serial interfaces working

**Non-Goals:**
- Full BMP ADIV5 protocol stack integration (future work)
- GDB RSP server (future work)
- Flash stub upload to target RAM (future work)
- JTAG support (SWD only for now)
- FreeRTOS integration (bare metal is fine)

## Decisions

### D1: PIO Sub-Program Architecture (from MioLink)

Use 7 modular PIO sub-programs with `.origin` directives instead of monolithic command dispatch:

```
PIO Memory Layout:
┌──────────────────────────────────────────────┐
│ origin 0:  start (wait for command)           │
│ origin 1:  seq_out_turnaround (float→drive)   │
│ origin 2:  seq_out (write bits to SWDIO)      │
│ origin 7:  seq_in_turnaround (drive→float)    │
│ origin 8:  seq_in (read bits from SWDIO)      │
│ origin 14: adiv5_check_ack (hw ACK validation)│
│ origin 21: turnaround_float_to_drive          │
└──────────────────────────────────────────────┘
```

C code calls sub-programs by loading origin address into OSR and executing `out pc, 5`.

### D2: Pin Mapping (RP2350 PIO1)

- SWCLK: GPIO4 (side-set pin 0)
- SWDIO: GPIO5 (in/out/set pin)
- Side-set: 1 bit (SWCLK only, no level shifter)
- Note: MioLink uses 2-bit side-set for level shifter control; we use 1-bit since we don't have external level shifters

### D3: DMA Integration

Adapt MioLink's `tap_pio.c` DMA layer for RP2350:
- DMA channel 0: Memory → PIO TX FIFO (send)
- DMA channel 1: PIO RX FIFO → Memory (receive)
- Use DMA request (DREQ) pacing from PIO SM
- RP2350 DMA is compatible with RP2040 DMA API

### D4: Turnaround Timing

Dedicated turnaround sub-programs at origins 1, 7, and 21:
- `seq_out_turnaround`: Sets SWDIO to output before write sequence
- `seq_in_turnaround`: Sets SWDIO to input before read sequence  
- `turnaround_float_to_drive`: Switches direction without data transfer

This eliminates the turnaround mismatch because turnaround is handled by hardware, not C code assumptions.

### D5: Dormant-to-SWD Sequence

Add dormant wake sequence from BMP's `adiv5_swd.c`:
1. Line reset: 60 HIGH cycles + 4 idle
2. JTAG-to-SWD: 0xE79E (16 bits)
3. Line reset again: 60 HIGH cycles + 4 idle
4. Dormant-to-SWD: 128-bit Selection Alert + activation code

This will be implemented in C code (not PIO) since it's a one-time initialization sequence.

### D6: SWD Frequency

- PIO clock: 125 MHz (RP2350 default)
- Max SWD freq: sys_clk / 8 = 15.625 MHz (with [3] delay cycles)
- Default SWD freq: 1 MHz (divider = 125)
- Configurable via `swd_set_clock()`

## Risks / Trade-offs

### R1: PIO Instruction Space
MioLink's 7 sub-programs use ~25 PIO instructions. RP2350 PIO has 32 instructions per SM. This fits but leaves little room for future extensions.
**Mitigation:** Use PIO1 SM0 (dedicated to SWD), leaving other SMs free.

### R2: DMA Channel Usage
DMA channels 0-1 for SWD. We already use DMA for logic analyzer capture on PIO0.
**Mitigation:** PIO0 capture and PIO1 SWD use different DMA channels; no conflict.

### R3: No BMP Protocol Stack
We're adopting MioLink's PIO architecture but not the full BMP ADIV5 protocol stack.
**Mitigation:** Keep our existing SWD CLI commands (`SWD IDCODE`, `SWD READ`, `SWD WRITE`). Add BMP protocol later as separate change.

### R4: RP2350 vs RP2040 Differences
MioLink targets RP2040. Key differences:
- RP2350 has PIO0 + PIO1 (RP2040 has PIO0 + PIO1 too)
- RP2350 PIO instruction set is compatible
- RP2350 DMA API is compatible
- RP2350 runs at up to 150 MHz (vs 125 MHz RP2040)
**Mitigation:** PIO and DMA APIs are forward-compatible; minimal changes needed.
