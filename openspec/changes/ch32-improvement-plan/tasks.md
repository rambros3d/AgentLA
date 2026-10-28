## 1. SWIO PIO Engine

- [x] 1.1 Create `firmware/pio/swio.pio` — Port PicoRVD's singlewire.pio program
- [x] 1.2 Create `firmware/src/programmer/swio.c` — PIO wrapper with `swio_init()`, `swio_put()`, `swio_get()`, `swio_reset()`
- [x] 1.3 Create `firmware/src/programmer/swio.h` — Public API header
- [x] 1.4 Update `CMakeLists.txt` — Add swio.pio to pico_generate_pio_header
- [x] 1.5 Update `ch32.c` — PIO init in ch32_probe_target()
- [x] 1.6 Update `riscv_dmi.c` — Use PIO-based `swio_put()`/`swio_get()` for SWIO mode
- [x] 1.7 Verify PIO1 resource allocation — SM0 for SWD, SM1 for SWIO, no conflicts

## 2. Manual MCU Selection (replaces auto-detect)

- [x] 2.1 Remove `ch32_auto_detect()` from `ch32.c` and its declaration from `ch32.h`
- [x] 2.2 Add MCU model table + `ch32_lookup_mcu()` / `ch32_probe_mcu()` in `ch32.c`/`ch32.h` (ch32v003→SWIO, ch32v203/ch32v307→RVSWD)
- [x] 2.3 Fix `ch32_probe_target()` false positive — require verified DMSTATUS link, ESIG=0 no longer fabricates chip_id
- [x] 2.4 Make RVSWD PIO puts bounded — stalled state machine can't hang firmware (SWIO→RVSWD transition hang)
- [x] 2.5 Add proper SBA read/write (SBCS config + busy polling) in `riscv_dmi.c` — fixes ESIG=0 reads
- [x] 2.6 Update `cmd_ch32_detect()` in `main.c` — `mcu=` selection, ARM routing, legacy `mode=` preserved
- [x] 2.7 Hardware test — `CH32 DETECT mcu=ch32v203` on real hardware — **PASSED 2026-08-04** (see Progress)

## 3. PROGBUF Flash Injection

- [x] 3.1 Create `firmware/src/programmer/ch32_progbuf.c` — PROGBUF flash write implementation
- [x] 3.2 Create `firmware/src/programmer/ch32_progbuf.h` — Public API header
- [x] 3.3 Implement `ch32_progbuf_flash_write()` — Load PROGBUF program, configure registers, stream data
- [x] 3.4 Implement `ch32_progbuf_flash_read()` — Read back via SBA
- [x] 3.5 Update `ch32.c` — Use PROGBUF path when available, fall back to simple writes
- [ ] 3.6 Test PROGBUF flash write on CH32V003 hardware — requires hardware

## 4. CH32V20x/V30x Flash Support

- [x] 4.1 Add CH32V20x flash register definitions to `ch32.h`
- [x] 4.2 Implement `ch32_v20x_flash_unlock()` — Different key sequence for V20x
- [x] 4.3 Implement `ch32_v20x_flash_erase()` — 256-byte page erase
- [x] 4.4 Implement `ch32_v20x_flash_program()` — V20x-specific programming sequence
- [x] 4.5 Update `ch32_probe_target()` — Detect V20x/V30x and configure appropriate flash algorithm
- [x] 4.6 Fix page_size to 256 for CH32V203 (was 128) — confirmed via minichlink reference
- [x] 4.7 Route `ch32_flash_program()` by chip_type — V203/V307 → `ch32_v20x_flash_program()` (256-byte pages), V003 → PROGBUF (64-byte pages). **Fixes flash write on V203** (was routing to V003-specific PROGBUF path)

## 5. Integration & Testing

- [x] 5.1 Update `main.c` — Add PROGBUF and SWIO diagnostic commands
- [x] 5.2 Build verification — `cmake --build build` succeeds with no errors
- [ ] 5.3 Hardware test — CH32V003 SWIO detection + flash write via PIO — requires hardware
- [x] 5.4 Hardware test — CH32V203 SWD detection + flash write — **PASSED 2026-08-04** (detect + erase + 32-byte write; see Progress for readback gap)
- [ ] 5.5 Performance measurement — Compare PIO vs bit-bang flash write speeds — requires hardware

## 7. Blockers Discovered During Hardware Testing

- [x] 7.1 Add `CH32 READ <addr> <len>` CLI command — readback verification currently impossible (`FLASH READ` routes to ARM SWD, not CH32/RVSWD SBA path)
- [x] 7.2 Fix CDC RX truncation at 256 bytes — **Diagnostic instrumentation added** (per-stage byte counters, DIAG CDC enhanced, DIAG PIPELINE test, 256-byte read buffer, periodic tud_task during accumulation). Deploy firmware and run DIAG PIPELINE + send test string to pinpoint loss location.
- [ ] 7.3 Re-run full hardware verification suite (`test_ch32v203_hardware_verification.py`) after 7.1 + 7.2 — readback parity check is Stage 5 of the suite

## Current Progress (2026-08-04)

### Verified on hardware (CH32V203, AgentLA RP2350, firmware build 2026-08-04 01:04:13)

**Working:**
- `CH32 DETECT mcu=ch32v203` → RVSWD link up (DMSTATUS=0x00000382), canonical `chip_id=0x20305040` from model table, `flash_size=65536 page_size=256 mode=2wire`. No hang, no false positive.
- `CH32 ERASE 0x08000000 256` → `OK CH32 ERASED 256 bytes`
- `CH32 FLASH 0x08000000 <32-byte payload>` → `OK CH32 FLASH WROTE 32 bytes` (after task 4.7 routing fix; previously failed via V003-specific PROGBUF path)
- `CH32 READ 0x08000000 <len>` → reads flash via SBA, returns comma-separated hex (task 7.1)
- `CH32 DETECT mcu=ch32v203` is repeatable across boots (verified 5× during recovery testing)

**Pending hardware validation:**
- `CH32 READ` readback parity (128-byte write/read check) — test scripts updated, ready to run
- CDC 256-byte truncation — diagnostic counters deployed, needs hardware test to pinpoint loss location
- CDC pipeline: enhanced DIAG CDC + DIAG PIPELINE commands available for testing

**Not working / gaps:**
- **Readback**: `CH32 READ <addr> <len>` now exists (task 7.1) and reads via SBA. Needs hardware validation.
- **CDC 256-byte truncation**: Diagnostic instrumentation deployed (task 7.2). Per-stage counters + DIAG CDC/Pipeline commands ready for hardware test to pinpoint loss location.
- **Target goes dark after erase**: Erasing 0x08000000 (target app code) left the CH32V203 unresponsive until AgentLA reboot. RVSWD re-initialization on the next DETECT recovers it.

### Other notes
- AgentLA boot counter: flashing a build with a new BUILD_TIMESTAMP resets the crash-loop counter (`BOOTS:1/3`). Stale timestamps (rebuild without reconfiguring CMake) caused a BOOTSEL lockout — recovery = reconfigure + reflash (confirmed 2026-08-04: reconfigure → build hash change → counter reset → `BOOTS:1/3`).
- PROGBUF flash path (task 3.x) is verified functional only for V003 at the code level; V203 routes to `ch32_v20x_flash_program()` per task 4.7. PROGBUF on V203 is initialized (`PROGBUF supported, initializing`) but not used for flash writes.

### Session log 2026-08-04 (diagnostics)
1. Recovered device from BOOTSEL lockout (stale BUILD_TIMESTAMP → reconfig + reflash) → `VERSION:2026-08-04 01:04:13`, `BOOTS:1/3`.
2. Re-verified DETECT/ERASE/32-byte WRITE on new build — all pass.
3. 128-byte payload test → `ERR unknown command` with only the tail in cmd_buffer → RX loss proven at device level.
4. Boundary mapping (200→428 chars) → loss is exactly first 256 bytes, deterministic.
5. Excluded: host burst coalescing (slow sends), pyserial (raw os.write), TinyUSB FIFO overflow (1024 verified), app ring overflow (1024).
6. Suspect: DCD/DPRAM layer or a 256-byte boundary in USB controller buffering. **Pending: instrument firmware (RX counters + DIAG CDC) to find where the first 256 bytes are consumed.**
## 6. Hardware Verification Preparation

- [x] 6.1 Update `test_ch32v203_swd_blink.py` — page_size check updated to 256
- [x] 6.2 Create `docs/ch32v203-test-instructions.md` — wiring, flash, run test steps
- [x] 6.3 Verify test scripts parse CH32 DETECT response correctly
- [x] 6.4 Verify test scripts handle 256-byte page erase/write operations
