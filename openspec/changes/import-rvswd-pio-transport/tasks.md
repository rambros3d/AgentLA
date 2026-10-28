# Tasks: Import RVSWD PIO Transport & CH32V203 Hardware Verification

> **STATUS: IN PROGRESS** (2026-08-14). F8/F9 firmware bugs fixed, probe reflashed, RAM-stub unprotect **VERIFIED** (RDP cleared, WPR unprotected — see `design.md` F10). Remaining blockers are **hardware-only**: the CH32V203's main-flash **erase** is dead — writes stick but erase never clears cells (see `design.md` F13) — so flash program/verify and the V203 completion gate cannot be finished on this chip. A healthy V203 is required to close sections 5.2/5.4.

## 1. Transport Vendor & PIO Driver Setup

- [x] 1.1 Create `firmware/src/programmer/rvswd/` directory structure
- [x] 1.2 Vendor `references/RVSWD_pico/RVSWD_full.pio` $\rightarrow$ `firmware/src/programmer/rvswd/rvswd.pio`
- [x] 1.3 Port `references/RVSWD_pico/RVSWD_pio.c` & `rvswd.h` $\rightarrow$ `firmware/src/programmer/rvswd/rvswd_pio_transport.c` and `rvswd_pio_transport.h`
- [x] 1.4 Implement non-blocking bounded FIFO put/get routines (`rvswd_put_bounded`) with 1ms deadline
- [x] 1.5 Update `CMakeLists.txt` to compile `rvswd.pio` header and link `rvswd_pio_transport.c`

## 2. Dynamic PIO0 SM0 Allocation & Pin Integration

- [x] 2.1 Implement dynamic program unloading/loading between ARM SWD (`probe.pio`) and RVSWD (`rvswd.pio`) on `PIO0 SM0`
- [x] 2.2 Wire `swd_deinit()` and `rvswd_deinit()` to call `pio_remove_program()` for clean instruction memory management
- [x] 2.3 Ensure `PROG_MODE_IS_ACTIVE()` protection wraps RVSWD `PIO0` usage to prevent collisions with Logic Analyzer capture mode
- [x] 2.4 Verify build succeeds cleanly without PIO memory allocation errors

## 3. RISC-V DMI & CH32 Debug Control Layer

- [x] 3.1 Connect `riscv_dmi_read()` and `riscv_dmi_write()` in `firmware/src/programmer/riscv_dmi.c` to the new RVSWD transport
- [x] 3.2 Add retry logic (up to 5 retries) for DMI register access failures
- [x] 3.3 Implement `ch32v20x_halt_microprocessor()`, `ch32v20x_resume_microprocessor()`, and `ch32v20x_reset_microprocessor_and_run()` in `ch32.c`
- [x] 3.4 Wire CPU register read/write (`COMMAND` / `DATA0` abstract command interface) for CH32V203

## 4. Primary Verification Phase 1: ARM SWD BluePill Regression Sweep

- [x] 4.1 Build firmware and flash test probe
- [x] 4.2 Run `test_swd_hardware.py`: Verify 12/12 hardware SWD tests PASS (SWD IDCODE `0x1BA01477`, DP/AP access, SWD reset)
- [x] 4.3 Run `test_stm32f1_hardware_suite.py`: Verify 5/5 TIERS PASS (full flash erase $\rightarrow$ program 9920 bytes $\rightarrow$ byte-exact verify $\rightarrow$ reset $\rightarrow$ boot)
- [x] 4.4 Confirm zero regressions to ARM SWD capability before proceeding to CH32 hardware validation

## 5. Primary Verification Phase 2: CH32V203 Hardware Read, Flash & Debug Validation

- [~] 5.1 **CH32V203 Target Detection & Read** — **PARTIAL — target DETECTs but was READ-PROTECTED**:
  - [x] Connect probe to CH32V203 dev board (GPIO4=SWCLK, GPIO5=SWDIO, GND)
  - [x] Run `CH32 DETECT mcu=ch32v203` — works after target power-cycle: `DMSTATUS=0x00000C82`, `chip_id=0x20310500`, `flash=64KB`, `page=256B`
  - [x] Region sweep (`test/test_rvswd_region_sweep.py`): ESIG/option-bytes/SRAM/peripherals readable; main flash (`0x08000000`) AND bootloader (`0x1FFF8000`) read fault `cmderr=3`
  - [x] Root cause: **chip shipped READ-PROTECTED** — `FLASH_OBR=0xFE` (RDPRT=1), `RDPR=0xFF`. Unprotect value is `RDP_Key=0x00A5` → `0x5AA5`. See `design.md` F1/F6/F10.
- [~] 5.2 **CH32V203 Flash Operations** — **BLOCKED (hardware): main-flash array erase is dead on this chip** (see `design.md` F13):
  - [x] Read-protection confirmed via OBR/RDPR (see 5.1); flash reads fault under RDP (this supersedes the earlier `0x4000B0C3` read-back finding)
  - [x] Halted-core debug unprotect attempts (`test/test_rvswd_unprotect*.py`): every op `cmderr=0` but option array went `0xE339E339`, RDP stayed active — halted-core option programming does not stick on this chip
  - [x] RAM-stub recovery (`test/stub_unprotect.S` + `test/test_rvswd_ramstub_unprotect.py`): **VERIFIED** — stub loads via per-word POSTEXEC, RDP cleared (`OBR=0x03FFFCFC`), all blocks unprotected (`WPR=0xFFFFFFFF`), option array `0xC03F5AA5 / 0x00FF00FF / ...` (see `design.md` F10). F9 wedge gone; probe fully responsive throughout.
  - [ ] Program firmware binary payload to `0x08000000` (heartbeat, LED on PB21) — **IMPOSSIBLE on this chip: main-array erase never clears cells (writes stick but there are no erased cells; reads revert to `0xE339E339`); requires a healthy V203**
  - [ ] Read back flash content and verify 100% byte-exact match with binary — **blocked by un-erasable array**
- [~] 5.3 **CH32V203 Debug Operations** — **PARTIAL** (halt/registers/resume verified; the rest is blocked by the dead target):
  - [x] Issue `CH32 HALT` / `DEBUG HALT`: Verify CPU halts (`DMSTATUS` halted bits set)
  - [x] Read CPU registers (`R0`–`R15`, `PC`): Verify valid values returned (via GPR abstract reads)
  - [ ] Issue `DEBUG STEP`: Single-step instruction, verify `PC` increments — not completed on V203 (dead flash, board degraded); verify on a healthy V203
  - [~] Mutate CPU register / SRAM address: GPR write/read round-trip verified; SRAM read path verified address-dependent
  - [x] Issue `CH32 RESUME` / `DEBUG RUN`: Verify target resumes execution smoothly
- [ ] 5.4 Document all test results and mark change complete — blocked on 5.2 (needs healthy V203)

### Firmware bugs found & FIXED during this phase (see `design.md` F8/F9)
- [x] `riscv_read_mem()` discarded `wait_abstract_cmd_complete()` errors (`riscv_dmi.c`) → stale DATA0 after faulted abstract ops (e.g. flash read under RDP). Fixed: propagate the error, don't read DATA0.
- [x] `ch32_v20x_wait_flash_busy()` ignored `WRPRTERR`/`PGERR` → a write-protected flash silently "succeeded". Fixed: check/clear W1C error flags, return `CH32_ERR_FLASH`.
- [x] PIO SM wedge when the target stalls the bus during flash ops: bounded-FIFO timeouts now fully re-init the PIO SM (self-heal). The WCH target DM also wedges on ~30–80 consecutive AUTOEXECDATA streamed executions — all firmware streaming paths (`ch32_progbuf_flash_write()`, `get/set_block_aligned()`) converted to one explicit POSTEXEC per word (proven: 113-word stub load, no wedge).
- [x] Flash erase order fixed to minichlink's sequence (`CTLR=FTER` → `ADDR` → `CTLR=FTER|STRT`); the old order never started the erase on V203.
- [ ] (Pre-existing, documented) PROGBUF flash-WRITE can't work on V203 (`cmderr=3`, flash controller doesn't intercept debug-context writes) — dormant `ch32_sram_flash_write` SRAM-blob path is the intended fix (and is the mechanism the F13 tests used); on this chip moot anyway (array un-erasable).
