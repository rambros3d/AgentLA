## 1. Runtime Driver Selection

- [x] 1.1 Add `prog_driver_t` enum + `prog_driver` global in `programmer.h`/`programmer.c` (default `PROG_DRIVER_BITBANG`)
- [x] 1.2 Add `PROG DRIVER BITBANG|PIO` command handler in `firmware/src/main.c` with `OK PROG DRIVER ...` response
- [x] 1.3 Implement `enter_bitbang_mode()` / `exit_bitbang_mode()` pin ownership (GPIO4/5/6 ↔ PIO engines)
  - Implemented as `swd_bb_gpio_init()` (entered once per session from `swd_init()`, with the `PROG_MODE_IS_ACTIVE()` UART-conflict guard); SWIO/RVSWD bitbang backends still pending (tasks 4/5).

## 2. Bitbang SWD Transport

- [x] 2.1 Implement `swd_bb_transfer(request, rnw, *data)` in `swd.c` (request → turnaround → ACK → data/parity → idle, 2µs post-edge sampling, parity check)
- [x] 2.2 Add driver dispatch in `swd_transfer()` (and `swd_write_bits`/`swd_read_bits`/`swd_line_reset_ext`/`swd_jtag_to_swd` where needed)
- [x] 2.3 Restructure `swd_read_idcode()` to bitbang-first fast connect (read DPIDR first, ABORT/CTRLSTAT after, no 30-retry loop)
- [ ] 2.4 Build + flash; verify `SWD IDCODE` fast, `SWD READ DP 0x0` and `SWD WRITE DP 0x4` return OK
  - [BLOCKED] **BLOCKED.** `SWD IDCODE` is fast and DPIDR reads OK, but the first `SWD WRITE DP 0x4` (CTRL/STAT) after any fresh connect returns **FAULT ACK on the wire** (deterministic, invariant to data value and to the ACK→data turnaround hold). Because `swd_read_idcode()` ignores the write ACKs, the failed power-up leaves the DP un-powered and subsequent standalone transactions fail silent (`bb_ack=7`).
  - **Independent confirmation (2026-08-12):** Raspberry Pi **debugprobe** (CMSIS-DAP) built for the Pico 2 probe + pyOCD connect to the same Blue Pill **successfully** — full standard sequence (incl. the CTRL/STAT power-up write from a fresh state), 13/13 debugger feature sweep, and the heartbeat firmware flashed + executed. The STM32F1 SW-DP accepts the standard sequence and the working power-up value is `0x30000000`. **The FAULT is our bug** — a sticky flag set by an earlier phase of our bitbang write path (WDATAERR/STICKYORUN suspects). Full details + updated next steps in `design.md` → **Debugprobe sanity check (2026-08-12)** and **Session State & Root-Cause Handoff (2026-08-12)**.

## 3. STM32F1 Debugger & Flasher Verification

- [ ] 3.1 Verify `SWD TEST` → `OK SWD TEST PASS target=STM32 (Cortex-M3)` on bitbang — blocked on 2.4
- [ ] 3.2 Verify `DEBUG HALT` / `DEBUG RUN` / `DEBUG STEP` / `DEBUG REGS` / `DEBUG READ` / `DEBUG WRITE` — blocked on 2.4
- [ ] 3.3 Verify `FLASH ERASE` / `FLASH WRITE` / `FLASH READ` / `FLASH VERIFY` — blocked on 2.4
- [ ] 3.4 Run `test/test_stm32f1_hardware_suite.py` 5-tier suite to green under bitbang default — blocked on 2.4
- [ ] 3.5 Run `test/test_swd_hardware.py` to green — blocked on 2.4

## 4. Bitbang SWIO Transport (CH32)

- [ ] 4.1 Implement bitbang SWIO backend in `swio.c` (begin/data/complement/end framing, ~250kHz) + driver dispatch
- [ ] 4.2 Verify CH32 SWIO probe/read and programming commands on hardware
- [ ] 4.3 Run `test/test_ch32v203*.py` suite to green under bitbang default

## 5. Bitbang RVSWD Transport (RISC-V)

- [ ] 5.1 Implement bitbang RVSWD backend in `rvswd.c` mirroring `rvswd.pio` framing + driver dispatch
- [ ] 5.2 Verify RV target detection / DMI access on CH32V003 hardware
- [ ] 5.3 Run `test/test_ch32v003.py` to green under bitbang default

## 6. Cross-Engine & PIO Diagnostics

- [ ] 6.1 Verify CMSIS-DAP transfers over bitbang (`cmsis_dap.c` routes through swd API)
- [ ] 6.2 Verify `SWD PIO_IDCODE` and `SWD CAPTURE PIOREAD` still work after bitbang sessions (PIO mode preserved)
- [ ] 6.3 Run full regression: STM32F1 suite + CH32 suite + RV suite
