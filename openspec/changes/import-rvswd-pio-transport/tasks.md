# Tasks: Import RVSWD PIO Transport & CH32V203 Hardware Verification

> **STATUS: DEFERRED** (2026-08-14). Paused by user; will resume later.
> Resume points: (1) fix firmware bugs F8 (`riscv_read_mem()` stale DATA0) and F9 (PIO wedge) in `design.md`, (2) reflash probe, (3) retry RAM-stub unprotect (`test/stub_unprotect.S` + `test/test_rvswd_ramstub_unprotect.py`), (4) flash heartbeat on PB21. Target chip is currently RDP-protected with a corrupted option array (see F6).

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

- [~] 5.1 **CH32V203 Target Detection & Read** — **PARTIAL — target DETECTs but is READ-PROTECTED**:
  - [x] Connect probe to CH32V203 dev board (GPIO4=SWCLK, GPIO5=SWDIO, GND)
  - [x] Run `CH32 DETECT mcu=ch32v203` — works after target power-cycle: `DMSTATUS=0x00000C82`, `chip_id=0x20310500`, `flash=64KB`, `page=256B`
  - [x] Region sweep (`test/test_rvswd_region_sweep.py`): ESIG/option-bytes/SRAM/peripherals readable; main flash (`0x08000000`) AND bootloader (`0x1FFF8000`) read fault `cmderr=3`
  - [x] Root cause: **chip shipped READ-PROTECTED** — `FLASH_OBR=0xFE` (RDPRT=1), `RDPR=0xFF`. Unprotect value is `RDP_Key=0x00A5` → `0x5AA5`. See `design.md` F1/F6-F10.
- [ ] 5.2 **CH32V203 Flash Operations** — **BLOCKED (hardware): chip is read-protected and unprotect attempts corrupted the option array**:
  - [ ] Read-protection confirmed via OBR/RDPR (see 5.1); flash reads fault under RDP (this supersedes the earlier `0x4000B0C3` read-back finding)
  - [ ] Halted-core debug unprotect attempts (`test/test_rvswd_unprotect*.py`): every op `cmderr=0` but option array is now `0xE339E339`, `OBR=0x03FFFFFF` (OPTERR=1), RDP still active — halted-core option programming does not stick on this chip
  - [~] RAM-stub recovery (`test/stub_unprotect.S` + `test/test_rvswd_ramstub_unprotect.py`): stub loads + verifies via PROGBUF AUTOEXECDATA, PC/SP set works — **blocked by probe wedge (F9)** whenever the bus is touched during the stub's flash-op window
  - [ ] Program firmware binary payload to `0x08000000` (heartbeat, LED on PB21) once unprotected
  - [ ] Read back flash content and verify 100% byte-exact match with binary
- [~] 5.3 **CH32V203 Debug Operations** — **PARTIAL**: `test/test_rvswd_hardware.py` passes **4/4** (DMI reads real values, halt works `DMSTATUS 0x000C0382`, resume works `0x000C0C82`):
  - [x] Issue `CH32 HALT` / `DEBUG HALT`: Verify CPU halts (`DMSTATUS` halted bits set)
  - [x] Read CPU registers (`R0`–`R15`, `PC`): Verify valid values returned (via GPR abstract reads)
  - [ ] Issue `DEBUG STEP`: Single-step instruction, verify `PC` increments
  - [~] Mutate CPU register / SRAM address: GPR write/read round-trip verified; SRAM read path verified address-dependent
  - [x] Issue `CH32 RESUME` / `DEBUG RUN`: Verify target resumes execution smoothly
- [ ] 5.4 Document all test results and mark change complete — blocked on 5.1/5.2

### Open firmware bugs to fix (blocking flash work; see `design.md` F8/F9)
- [ ] `riscv_read_mem()` discards `wait_abstract_cmd_complete()` errors (`riscv_dmi.c:216/224/232`) → stale DATA0 (DETECT shows garbage chip_id after errors)
- [ ] PIO SM wedge when the target stalls the bus during flash ops (needs robust re-init/recovery, or the RVSWD transport must detect and self-heal)
