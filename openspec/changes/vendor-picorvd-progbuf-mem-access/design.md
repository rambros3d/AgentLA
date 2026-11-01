## Context

AgentLA's abstract-command memory access currently lives in `firmware/src/programmer/riscv_dmi.c`. `riscv_read_mem()` uses a `lw a0, 0(a1)` PROGBUF program and `riscv_write_mem()` uses a `sw s0, 0(a1)` program; both clobber two or three GPRs (a0/a1 and s0), require separate loads per direction, and do word-at-a-time accesses with no pipelining. `ch32_progbuf.c` already ports PicoRVD's flash-write streamer and a block-read program, but the block-read program's DTM DATA0/DATA1 magic offsets were hard-patched to CH32V203 values (`0x380`/`0x384`) — the CH32V003 single-wire SWIO family still needs the original `0xF4`/`0xF8` offsets.

PicoRVD (`references/picorvd/src/RVDebug.cpp`) is MIT-licensed and contains the canonical patterns: `prog_get_set_u32` (16 halfwords, one program for both read/write, direction via DATA1 bit 0, only clobbers A0/A1) and `get_block_aligned`/`set_block_aligned` (8 words each, `ABSTRACTAUTO_AUTOEXECDATA` pipelining).

Two distinct concepts must stay separate: **`g_ch32_target.chip_type` is a chip property** (its DTM's DATA0/DATA1 magic offsets, its flash controller, its page size — the code already dispatches flash logic on `chip_type`, see `ch32.c:287/331`), while **`g_ch32_target.mode` is a transport property** (which wire protocol: SWIO or RVSWD). The magic offsets are DTM/chip properties and must be keyed on `chip_type`; `mode` selects the transport only. Today the families map 1:1 to transports (V003↔SWIO, V20x↔RVSWD) but that is coincidental.

There are exactly three wire protocols in this firmware, each with its own module: **SWIO** (1-wire, V003, `swio.c`/`swio.pio`), **RVSWD** (2-wire, V203/V307, `rvswd.c`/`rvswd_pio_transport.c`), and **ARM SWD** (Cortex-M F103, `swd.c`/`target.c`, the `DEBUG` command family). ARM SWD is deliberately NOT part of `ch32_mode_t`: it lives in its own module and command family, and a third enum value with no producer would create dead dispatch paths.

Constraints: the public API (`riscv_read_mem`, `riscv_write_mem`, `riscv_write_hword`, `ch32_progbuf_flash_read`, etc.) is consumed by `ch32.c` and the CLI; wire protocols (1-wire SWIO vs 2-wire RVSWD) must not change; no GDB/SoftBreak features are in scope.

## Goals / Non-Goals

**Goals:**
- Port `prog_get_set_u32` as a single combined get/set aligned memory access (A0/A1-only clobber).
- Port `get_block_aligned` and `set_block_aligned` using explicit per-word `POSTEXEC` (avoiding `ABSTRACTAUTO_AUTOEXECDATA` streaming which wedges WCH silicon after ~30-80 executions).
- Centralize per-family DTM magic offsets in a single lookup keyed on `chip_type`: CH32V003 `0xF4`/`0xF8`, CH32V20x/V30x `0x380`/`0x384`.
- Rename the transport enum to `CH32_MODE_SWIO`/`CH32_MODE_RVSWD` and make mode dispatch exhaustive (switch + default error).
- Route aligned reads/writes in `riscv_dmi.c` through the vendored single-program path; use block-aligned read for flash readback.
- Carry MIT attribution headers consistent with existing ports.

**Non-Goals:**
- No changes to the SWIO/RVSWD wire protocols or PIO engines.
- No `CH32_MODE_ARM_SWD` enum value and no unification of the `DEBUG`/`CH32` command worlds (deferred).
- No porting of `SoftBreak.cpp` / `GDBServer.cpp` / register caching.
- No new GDB support.
- Not changing the flash-write streamer (`ch32_progbuf_flash_write`) which already works.

**Completion gate:** The change is complete only after the hardware verification tasks (CH32V003 read/flash/debug over SWIO with `0xF4`/`0xF8` offsets; CH32V203/V307 readback parity with `0x380`/`0x384` offsets) pass on real silicon. Implementation may proceed without hardware, but the change stays open until then.

## Decisions

**D1. Port the program tables verbatim, then parameterize offsets via a chip-family lookup (NOT transport mode).**
PicoRVD's `prog_get_set_u32` uses `lui a0,0xE0000` + `addi a0,a0,0xF4` and reads DATA1 at `4(a0)`. On CH32V20x/V30x the base differs (`0x380`/`0x384`). We keep one copy of each program and build the immediate offsets at codegen time from a single per-family lookup keyed on `g_ch32_target.chip_type`: `CH32_CHIP_V003` → 0xF4/0xF8 (PicoRVD's original values), `CH32_CHIP_V203`/`CH32_CHIP_V307` → 0x380/0x384 (HARTINFO-derived, matching the existing `ch32_progbuf.c` patch and the runtime formula `0xE0000000 | (hartinfo & 0x7FF)` used by the flash streamer). `g_ch32_target.mode` selects the transport and must NOT be used as the offset key — the offsets are a DTM/chip property, not a wire-protocol property. Alternatives rejected: keying on `mode` (conflates transport with chip DTM), two full program copies (duplication, drift risk).

**D2. `riscv_read_mem`/`riscv_write_mem` keep their public signatures; internal aligned fast path.**
`riscv_read_mem(addr, *val)` and `riscv_write_mem(addr, val)` remain the public entry points. The SBCS probe (SBA path) is preserved unchanged. For aligned addresses they call `ch32_progbuf_get_set_u32`; unaligned addresses fall back to the existing `lw`/`sw` PROGBUF programs (current behavior — there is no byte-level fallback in `ch32.c`; all existing callers pass aligned addresses, but the fallback keeps behavior identical for any future unaligned caller). Halt-if-needed / resume semantics and return codes are preserved. This avoids touching every call site in `ch32.c` and the CLI. Alternative rejected: add new API and migrate all callers (churn, no behavioral gain).

**D3. Block read becomes the flash readback path.**
`ch32_progbuf_flash_read` (already block-aligned, `ch32_progbuf.c:419`) is refactored to call the new vendored `ch32_progbuf_get_block_aligned`; the current inline program load is replaced by the centralized program table. This satisfies the `ch32-flash-programmer` readback scenario and reuses one proven implementation.

**D4. `set_block_aligned` is added but only exercised where it beats existing paths.**
The flash-write streamer already pipelines via `AUTOEXECDATA`; `set_block_aligned` is primarily for SRAM writes and future GDB-free block writes. It is ported for completeness and used by `riscv_write_mem` on aligned multi-word callers if any, otherwise kept available via the new API.

**D5. Attribution header added to `ch32_progbuf.c`.**
Program tables are grouped in a new section with a header crediting PicoRVD / aappleby / MIT, matching the existing `swio.c`/`swio.pio` convention. The mem-access functions live in `ch32_progbuf.c` next to the existing PicoRVD ports (resolved open question: no new file).

**D6. Transport enum renamed; mode dispatch made exhaustive.**
`CH32_MODE_1WIRE_SDI` → `CH32_MODE_SWIO` and `CH32_MODE_2WIRE_SWD` → `CH32_MODE_RVSWD`, so the transport enum names the protocols unambiguously (the old "2WIRE_SWD" name collides with ARM SWD, a different protocol in `swd.c`). ARM SWD is intentionally NOT added to `ch32_mode_t` — it has its own module and `DEBUG` command family; a value with no producer would silently fall into the RVSWD branch. Mode dispatch in `riscv_dmi.c` (`riscv_dmi_read`/`riscv_dmi_write`) and `ch32_probe_target()` converts from `if/else` to `switch` with a `default: error`, so an unknown transport fails loudly instead of being routed to RVSWD. CLI tokens (`1wire`/`2wire`) stay unchanged.

## Risks / Trade-offs

- [CH32V003 hardware untested for this change] → The `0xF4`/`0xF8` offsets are PicoRVD's shipped values used on real V003 silicon; the block-read path is already validated on V203. V003 read/flash/debug over SWIO is a **mandatory completion-gate task**, not optional.
- [Changing the hot mem path could regress V203 flash] → Keep `ch32_progbuf_flash_write` untouched; only the readback path changes. Validate via existing readback parity tests before/after.
- [ABSTRACTAUTO hardware wedge on WCH DM] → WCH Debug Module hardware wedges after ~30-80 AUTOEXECDATA streamed executions (discovered in F9 of `import-rvswd-pio-transport`). Mitigated by switching block access routines (`ch32_progbuf_get_block_aligned` / `set_block_aligned`) to explicit per-word `POSTEXEC` executions.
- [Unaligned accesses] → The vendored programs require 4-byte alignment; all callers are checked at the wrapper boundary, unaligned falls back to prior behavior.

## Migration Plan

0. Rename the transport enum (`CH32_MODE_1WIRE_SDI`→`CH32_MODE_SWIO`, `CH32_MODE_2WIRE_SWD`→`CH32_MODE_RVSWD`) and convert mode dispatch to switch/default-error. Mechanical, zero behavior change, landed first so no new code spells the old names.
1. Add the DTM offset lookup (keyed on `chip_type`) and program tables to `ch32_progbuf.c` with attribution.
2. Add `ch32_progbuf_get_set_u32`, `ch32_progbuf_get_block_aligned`, `ch32_progbuf_set_block_aligned` to `ch32_progbuf.h`.
3. Rewire `riscv_read_mem`/`riscv_write_mem` aligned path (SBA probe preserved, unaligned falls back to existing programs); refactor `ch32_progbuf_flash_read` to the shared block-read.
4. Build, flash, run V203 readback parity + existing `ch32` CLI tests.
5. **Run mandatory hardware verification (completion gate):** CH32V003 read, flash, and debug over single-wire SWIO (0xF4/0xF8); CH32V203/V307 readback parity (0x380/0x384). Record hardware/toolchain.
6. Rollback: keep old programs reachable via a fallback flag; revert the wrapper routing if parity fails.

## Open Questions

- ~~Should `set_block_aligned` be exposed via a CLI command now, or kept API-only until a consumer exists?~~ **Resolved: API-only.** No current consumer (the SRAM flash writer uses its own mechanism; GDB is out of scope).
- ~~Where should the centralized program tables live — `ch32_progbuf.c` or a new `ch32_picorvd_progs.c`?~~ **Resolved: `ch32_progbuf.c`**, next to the existing PicoRVD ports.
- **Deferred: unifying the `DEBUG` (ARM SWD) and `CH32` (SWIO/RVSWD) command worlds.** When/if that happens, introduce a shared transport concept that includes ARM SWD; out of scope here.
