## Why

The PIO-based transports (SWD on PIO0, SWIO/RVSWD on PIO1) still have a cross-transaction boundary bug: a standalone transaction after a completed sequence gets a silent all-ones ACK (`raw_ack=0xE0000000`), even though the connect + DPIDR read works reliably. Every layer above the transport (`target_detect`, `DEBUG HALT/STEP/REGS/READ/WRITE`, `FLASH ERASE/WRITE/VERIFY`, CMSIS-DAP) routes through those transports, so they all fail with `ERR 5` outside of a single fresh connect burst.

The bitbanging SWD reference driver is proven (100% working target ACK latching on the STM32F103 Blue Pill). We will ship the full debugger + flasher stack on a complete bitbang transport layer now, selectable at runtime, and defer finishing the PIO engines to a later change.

## What Changes

- Add a **runtime driver selection** (`PROG DRIVER BITBANG|PIO`, default BITBANG) at the programmer layer so the transport backends can be swapped without a rebuild.
- Implement a **full bitbang SWD transfer** (`request → turnaround → ACK → data/parity → idle`) supporting DP/AP reads AND writes — the existing `swd_bitbang_read_idcode()` only probes IDCODE.
- Route the existing protocol API (`swd_transfer`, `swd_read_dp/write_dp`, `swd_read_ap/write_ap`) through the bitbang backend when selected.
- Restructure the SWD connect sequence to be bitbang-first and fast (line reset → switch → **DPIDR read first** → ABORT/CTRLSTAT writes), removing the slow PIO retry chain that causes multi-second IDCODE reads.
- Implement **bitbang SWIO** (WCH CH32 1-wire) and **bitbang RVSWD** (WCH RISC-V) transports mirroring `swio.pio` / `rvswd.pio` framing, covering the CH32/RV debugger + flasher paths.
- Keep PIO mode available for the `SWD CAPTURE` logic-analyzer diagnostic; bitbang mode owns GPIO4/5 as SIO pins for the duration of a programmer session.

## Capabilities

### New Capabilities
- `stm32f1-bitbang-debugger`: Defines requirements for a runtime-selectable bitbang transport layer covering SWD (STM32F1), SWIO (CH32), and RVSWD (CH32V003/RISC-V) debuggers and flashers, with bitbang as the default driver while PIO integration is deferred.

### Modified Capabilities
- None.

## Impact

- `firmware/src/programmer/programmer.c` / `programmer.h`: Global driver mode enum + `PROG DRIVER` command handling.
- `firmware/src/programmer/swd.c`: Full `swd_bb_transfer()`; driver dispatch in `swd_transfer`; fast bitbang connect in `swd_read_idcode`.
- `firmware/src/programmer/swio.c`: Bitbang SWIO transfer backend + dispatch.
- `firmware/src/programmer/rvswd.c`: Bitbang RVSWD transfer backend + dispatch.
- `firmware/src/main.c`: `PROG DRIVER` CLI registration; diagnostic strings.
- `firmware/pio/swd.pio`, `swio.pio`, `rvswd.pio`: Unchanged (PIO engines retained for later integration).
- `test/`: STM32F1 5-tier suite + CH32/RV tests run against the bitbang default.
