## Why

The MioLink-derived modular PIO SWD transport (7 origin-addressed sub-programs, autopull/autopush coupling, C-tracked direction) has consumed the entire debugging session — the deterministic CTRL/STAT write FAULT that blocks `stm32f1-bitbang-debugger` task 2.4 points at our PIO/write-path mechanics, and the debugprobe sanity check proved an independent PIO implementation (Raspberry Pi debugprobe + pyOCD) performs the identical ADIv5 sequence **successfully** on the same Blue Pill, pins, board, and SDK. Rather than keep debugging fragile in-house PIO machinery by eye, replace the fragile core with the proven one.

## What Changes

- **Vendor debugprobe's PIO SWD transport** (`probe.pio` + `probe.c` transport functions, both **MIT**; `SWD_Transfer` from `sw_dp_pio.c`, **Apache-2.0**) into `firmware/src/programmer/`. All licenses are GPL-3.0-compatible; attribution headers preserved.
- **Port `SWD_Transfer`** as the PIO transaction engine, extracted into a standalone function with a small config shim (no CMSIS-DAP `DAP_Data` dependency). This inherits the exact proven sequence: continuous-clock turnaround (read `turnaround + 3`, discard junk), WAIT/FAULT data-phase back-off, protocol-error re-sync, idle-cycle handling.
- **Rewire `swd_transfer()`'s PIO branch**: decode our wire request byte → debugprobe request layout → call ported `SWD_Transfer`. The bitbang branch (`swd_bb_transfer`) is **unchanged** and remains the fallback.
- **Delete the modular PIO architecture**: the 7 sub-programs in `firmware/pio/swd.pio`, `tap_pio.c`'s origin-stream/DMA dispatch, `current_swdio_dir` tracking, and all autopull/autopush filler-word workarounds. Replace with the vendored `probe.pio` + transport.
- **Switch the power-up value** from `0x50000000` to **`0x30000000`** (bit28|bit29) in both PIO and bitbang branches — the value pyOCD confirmed working.
- **Pins/board**: vendored code uses the same PIO0/SM0 on GPIO4/5 already defined in `swd.h`; keep the `PROG_MODE_IS_ACTIVE()` guard so the capture engine's PIO0 usage never overlaps.
- **Out of scope**: FreeRTOS, the CMSIS-DAP USB stack, `SWJ_Sequence`/`SWD_Sequence` (our line-reset/JTAG-to-SWD in `swd.c` is wire-proven), and the CH32 SWIO / RVSWD paths (separate PIO programs).

## Capabilities

### New Capabilities
- `debugprobe-pio-transport`: The vendored debugprobe PIO SWD transport — the `probe` PIO program, the `probe_write_bits` / `probe_read_bits` / `probe_hiz_clocks` / `probe_set_swclk_freq` primitives, and the ported `SWD_Transfer` transaction engine. Covers module boundaries, pin/pio allocation, clock setting, and the FIFO command discipline (explicit pull/push, direction in-band, continuous clock).

### Modified Capabilities
- `swd-engine`: The PIO transport requirements change — the modular seq_out/seq_in sub-program buffer structure is replaced by debugprobe's continuous-clock FIFO command model; the wire shape (continuous-clock turnaround vs. quiet-low turnaround) and error recovery (data-phase back-off vs. SM restart) are new requirements.

## Impact

- **Code**: `firmware/src/programmer/swd.c`, `swd.h`, `tap_pio.c`/`tap_pio.h` (deleted or reduced), `firmware/pio/swd.pio` (replaced by vendored `probe.pio`), `firmware/src/main.c` (power-up value call sites), `CMakeLists.txt` (PIO header generation), vendored new files under `firmware/src/programmer/debugprobe/`.
- **Specs**: `swd-engine` requirements rewritten for the new transport; new `debugprobe-pio-transport` spec.
- **API**: public `swd_transfer()` signature unchanged; `prog_driver` BITBANG/PIO dispatch unchanged; CLI and host SDK unaffected.
- **Dependencies**: debugprobe code vendored under MIT / Apache-2.0 (GPL-3.0-compatible). No new toolchain requirements.
- **Hardware validation targets**: STM32F103 Blue Pill (primary), CH32 targets via existing suites (regression only — their transports are untouched).
