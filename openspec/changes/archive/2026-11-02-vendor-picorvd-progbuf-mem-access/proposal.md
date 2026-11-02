## Why

PicoRVD is the canonical, field-proven reference implementation for WCH QingKe RISC-V debug. Its `RVDebug.cpp` and `WCHFlash.cpp` contain the optimal abstract-command memory-access patterns (a single combined get/set `prog_get_set_u32` program that only clobbers A0/A1, plus `ABSTRACTAUTO`-pipelined block reads/writes). AgentLA's `riscv_dmi.c` currently uses two separate per-direction PROGBUF programs that clobber three GPRs and lack the pipelined block path, so vendoring the proven PicoRVD patterns will shrink the abstract-command layer, reduce register clobbering, and speed up memory/flash reads. The SWIO transport itself and the flash-write streamer are already vendored; this change completes the picture.

## What Changes

- Add a new `ch32_progbuf_get_set_u32` abstract-command function ported from PicoRVD's `prog_get_set_u32` (16 halfwords, one program for both read and write, direction flagged by DATA1 bit 0, clobbers only A0/A1).
- Add a `ch32_progbuf_get_block_aligned` fast-read path and a `ch32_progbuf_set_block_aligned` fast-write path ported from PicoRVD's `get_block_aligned`/`set_block_aligned`, executing per-word `POSTEXEC` (avoiding `ABSTRACTAUTO_AUTOEXECDATA` streaming which wedges the WCH Debug Module hardware after ~30-80 executions).
- Parameterize the WCH DTM DATA0/DATA1 magic offsets per **chip family** (a DTM property, not a transport property) via a single lookup keyed on `g_ch32_target.chip_type`: **0xF4/0xF8** for CH32V003 and **0x380/0x384** for CH32V203/V307. This replaces the hard-patched values in `ch32_progbuf.c`.
- Rename the transport enum for clarity: `CH32_MODE_1WIRE_SDI` → `CH32_MODE_SWIO`, `CH32_MODE_2WIRE_SWD` → `CH32_MODE_RVSWD`, and make mode dispatch exhaustive (switch + default error). SWIO, RVSWD, and ARM SWD remain three distinct transports (ARM SWD stays in `swd.c`, intentionally not in the enum).
- Add license attribution headers to the ported program tables (MIT, aappleby/picorvd) matching the existing `swio.c`/`swio.pio`/`ch32_progbuf.c` convention.
- Keep the existing `riscv_read_mem`/`riscv_write_mem`/`riscv_write_hword` function signatures as the public API; internally route aligned accesses through the new single-program path.
- **Hardware verification is a completion gate.** The change SHALL NOT be marked complete until hardware verification passes on real silicon, covering: (1) CH32V003 read, flash programming, and debugging over the single-wire SWIO interface using the `0xF4`/`0xF8` offsets, and (2) CH32V203/V307 readback parity over 2-wire RVSWD using the `0x380`/`0x384` offsets. Implementation without hardware is tracked but the change stays open until the hardware tasks are done.

## Capabilities

### New Capabilities

- `ch32-progbuf-mem-access`: Vended PROGBUF-based aligned memory read/write (single combined get/set program clobbering only A0/A1) plus per-word POSTEXEC block read/write, with WCH DTM DATA0/DATA1 magic offsets centralized in a lookup keyed on chip family (0xF4/0xF8 for CH32V003, 0x380/0x384 for CH32V20x/V30x) — independent of the transport (SWIO vs RVSWD).

### Modified Capabilities

- `riscv-debug-module`: The abstract-command memory access requirements change to reference the vendored single-program + block-pipelined implementation (A0/A1-only clobber, direction via DATA1 bit 0) instead of the current separate read/write programs.
- `ch32-flash-programmer`: Flash readback verification shall use the vendored `get_block_aligned` fast-read path (as PicoRVD's `WCHFlash::verify_flash` does) rather than word-at-a-time reads.

## Impact

- Firmware: `firmware/src/programmer/riscv_dmi.c` — route aligned mem reads/writes through the vendored single-program path; exhaustive mode dispatch
- Firmware: `firmware/src/programmer/ch32_progbuf.c` — add `get_set_u32` / `get_block_aligned` / `set_block_aligned`, chip-family-keyed magic offset lookup; keep existing flash-write streamer
- Firmware: `firmware/src/programmer/ch32.h` — transport enum renamed to `CH32_MODE_SWIO`/`CH32_MODE_RVSWD` (mechanical; CLI tokens unchanged)
- Firmware: `firmware/src/programmer/riscv_dmi.h` — new/changed function declarations
- No wire-protocol changes: SWIO (1-wire) and RVSWD (2-wire) transports are untouched
- Reference: `references/picorvd/src/RVDebug.cpp` (MIT, aappleby) — source of ported program tables
- Test: extend `test_rvswd_ramstub_unprotect.py`-style flash readback verification to exercise the block-aligned read path
- Hardware: CH32V003 (1-wire SWIO, 0xF4/0xF8) and CH32V203/V307 (2-wire RVSWD, 0x380/0x384) boards — required for the completion gate
