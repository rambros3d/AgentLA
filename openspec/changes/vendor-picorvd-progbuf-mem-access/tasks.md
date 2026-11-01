## 0. Transport Enum Cleanup

- [x] 0.1 Rename `CH32_MODE_1WIRE_SDI`→`CH32_MODE_SWIO` and `CH32_MODE_2WIRE_SWD`→`CH32_MODE_RVSWD` in `ch32.h`/`ch32.c`/`riscv_dmi.c`/`main.c` (CLI tokens `1wire`/`2wire` unchanged) (D6)
- [x] 0.2 Convert mode dispatch in `riscv_dmi.c` (`riscv_dmi_read`/`riscv_dmi_write`) and `ch32_probe_target()` to `switch` with a `default: CH32_ERR_UNSUPPORTED` so an unknown transport fails loudly (D6)

## 1. Program Tables & Offsets

- [x] 1.1 Add a single per-family DTM DATA0/DATA1 magic offset lookup keyed on `g_ch32_target.chip_type` (`CH32_CHIP_V003` → 0xF4/0xF8; `CH32_CHIP_V203`/`CH32_CHIP_V307` → 0x380/0x384) in `ch32_progbuf.c` — a small table/function, NOT keyed on transport `mode` (D1)
- [x] 1.2 Port `prog_get_set_u32` (16 halfwords) from `references/picorvd/src/RVDebug.cpp:540` into `ch32_progbuf.c`, building the immediate offsets from the lookup (D1)
- [x] 1.3 Port `prog_get_block_aligned` (8 words) from `RVDebug.cpp:602` and `prog_set_block_aligned` (8 words) from `RVDebug.cpp:637`, parameterized by the lookup, replacing the hard-coded `prog_get_block_aligned` at `ch32_progbuf.c:408`
- [x] 1.4 Add MIT attribution header crediting PicoRVD (https://github.com/aappleby/picorvd) to the program table section (D5)

## 2. Vendored Memory-Access Functions

- [x] 2.1 Implement `ch32_progbuf_get_set_u32(uint32_t addr, uint32_t *val, bool write)` in `ch32_progbuf.c` ported from `get_mem_u32_aligned`/`set_mem_u32_aligned` (`RVDebug.cpp:567/583`): load prog, set DATA0/DATA1 (bit 0 = direction), POSTEXEC, wait busy
- [x] 2.2 Implement `ch32_progbuf_get_block_aligned(uint32_t addr, uint32_t *dst, int size_dwords)` ported from `get_block_aligned` (`RVDebug.cpp:598`): per-word explicit `POSTEXEC` execution to avoid WCH DM hardware lockups caused by `ABSTRACTAUTO_AUTOEXECDATA` streaming
- [x] 2.3 Implement `ch32_progbuf_set_block_aligned(uint32_t addr, const uint32_t *src, int size_dwords)` ported from `set_block_aligned` (`RVDebug.cpp:633`): per-word explicit `POSTEXEC` execution
- [x] 2.4 Declare the three new functions in `ch32_progbuf.h`
- [x] 2.5 Route `riscv_read_mem()`/`riscv_write_mem()` (`riscv_dmi.c:175/249`) aligned fast path through `ch32_progbuf_get_set_u32`; keep the SBCS probe (SBA path) and existing return semantics; unaligned addresses fall back to the existing `lw`/`sw` PROGBUF programs (D2)

## 3. Flash Readback Integration

- [x] 3.1 Refactor `ch32_progbuf_flash_read` (`ch32_progbuf.c:419`) to call `ch32_progbuf_get_block_aligned` instead of its inline program load loop (D3)
- [x] 3.2 Ensure flash readback respects `CH32V20x_FLASH_CTLR` lock/reset and `dcsr.ebreakm` handling as today

## 4. Build & Verification

- [x] 4.1 Rebuild firmware (`cmake --build <build> --target AgentLA`) — zero compiler warnings
- [ ] 4.2 Flash to AgentLA probe and run existing CH32V203 tests: `test_ch32v203_swd_blink.py` and/or `test_rvswd_ramstub_unprotect.py` — confirm flash program + readback parity matches pre-change baseline
- [ ] 4.3 Exercise `CH32 FLASH` CLI end-to-end on V203 (write → verify via block read)

## 5. Hardware Verification (Completion Gate)

> **STATUS: PENDING HARDWARE (2026-11-01)** — `import-rvswd-pio-transport` completed and archived on 2026-10-31. Firmware implementation (Sections 0–4.1) is complete and verified in code.
> Bench state: V203 DETECT/flash/ESIG-read pass on the new firmware (get/set path verified, `chip_id=0x20310500`). Block memory access operates via per-word `POSTEXEC` to prevent WCH Debug Module hardware wedges (F9).
> Target gates: CH32V003 single-wire SWIO verification (5.1–5.5) and CH32V203/V307 readback parity (5.6) on real target silicon.

> **Gate:** The change SHALL NOT be marked complete until ALL tasks in this section pass on real hardware. Implementation tasks (sections 1–4) may proceed without hardware, but the change stays open until this section is done.

- [ ] 5.1 CH32V003 over single-wire SWIO (offsets 0xF4/0xF8): `CH32 DETECT` identifies the target via SWIO
- [ ] 5.2 CH32V003 over SWIO: `CH32 READ` / memory read of ESIG (0x1FFFF7E0 FLACAP, 0x1FFFF7E8 UID) returns correct values
- [ ] 5.3 CH32V003 over SWIO: `CH32 FLASH 0x08000000 <hex>` programs firmware successfully, and readback verification via block-aligned read matches
- [ ] 5.4 CH32V003 over SWIO: halt, single-step, register read/write, and resume operate correctly (debugging over SWIO)
- [ ] 5.5 CH32V003 over SWIO: a flashed blink/GPIO program runs after reset (end-to-end debug → flash → run)
- [ ] 5.6 CH32V203/V307 over 2-wire RVSWD (offsets 0x380/0x384): re-run flash readback parity on real hardware with the vendored block-read path
- [ ] 5.7 Confirm the change is marked complete ONLY after 5.1–5.6 pass; record the hardware/toolchain used in the archive notes
