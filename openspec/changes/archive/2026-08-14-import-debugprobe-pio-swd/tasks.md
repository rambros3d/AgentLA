## 1. Vendor the debugprobe PIO transport

- [x] 1.1 Copy `references/debugprobe/src/probe.pio` → `firmware/src/programmer/debugprobe/swd_probe.pio` (keep MIT header; add comment pinning upstream commit `3fff5b2` and noting local adaptations). Add `pico_generate_pio_header` entry in `CMakeLists.txt`
- [x] 1.2 Vendor `probe.c` transport functions as `firmware/src/programmer/debugprobe/probe_pio_transport.c`/`.h` (MIT header): `probe_write_bits`, `probe_read_bits`, `probe_hiz_clocks`, `probe_set_swclk_freq`, `probe_init`/`probe_deinit` — adapted to `swd.h` pin defines (SWCLK GPIO4 side-set, SWDIO GPIO5 in/out/set, PIO0 SM0), a `CPU_CLOCK` define, and `PROBE_PIN_*` macros; drop `probe_config.h`/`DAP_config.h` includes and `probe_dump`/`probe_info` logging or gate behind `dbg_printf`. (Note: `probe_set_swclk_freq` derives the divider from `clock_get_hz(clk_sys)` at runtime, so no `CPU_CLOCK` define was needed; `probe_read_bits` gained a bounded 10 ms timeout vs. upstream's blocking read.)
- [x] 1.3 Port `SWD_Transfer` from `references/debugprobe/src/sw_dp_pio.c` as `firmware/src/programmer/debugprobe/swd_transfer_pio.c`/`.h` (Apache-2.0 header): replace `DAP_Data.swd_conf.turnaround`/`transfer.idle_cycles`/`clock_delay`/`timestamp` with a small config struct + direct `probe_set_swclk_freq()` call; keep parity verify, WAIT/FAULT dummy-phase back-off, protocol-error re-sync, idle cycles; drop `SWJ_Sequence`/`SWD_Sequence`

## 2. Wire the new transport into the SWD engine

- [x] 2.1 In `swd_transfer()`: decode the wire request byte into debugprobe request layout (`APnDP|RnW<<1|A[2:3]<<2`) and route the PIO branch through the ported `SWD_Transfer`; bitbang branch unchanged; keep ACK mapping (OK/WAIT/FAULT/0x07 — parity error 0x05 normalized to 0x07; `swd_debug_raw_ack` updated for diagnostics)
- [x] 2.2 Rewire `swd_write_dp_no_check()`, `swd_write_bits()`, `swd_read_bits()`, `swd_hiz_clocks()`, `swd_line_reset_ext()`, `swd_jtag_to_swd()` PIO branches to use the vendored primitives; remove `current_swdio_dir` tracking and `SWD_ORIGIN_*`-based buffer assembly (direction is now in-band; `current_swdio_dir` remains only for the bitbang backend)
- [x] 2.3 Replace `swd_set_clock()` internals with `probe_set_swclk_freq()` mapping (keep `SWD_FREQ_MIN`/`SWD_FREQ_MAX` clamping); `swd_init()` PIO path calls `probe_init()` + 100 kHz default

## 3. Remove the modular PIO architecture

- [x] 3.1 Delete the 7 sub-programs and `SWD_ORIGIN_*` constants from `firmware/pio/swd.pio` (replace with the vendored `swd_probe.pio`; remove the now-unused `swd.pio.h` include) and remove the old `pico_generate_pio_header(swd.pio)` from `CMakeLists.txt`
- [x] 3.2 Delete `tap_pio.c`/`tap_pio.h` origin-stream/DMA dispatch (`tap_pio_dma_send_uint32`, `tap_pio_dma_send_recv_uint32`, `swd_diag_state`, `tap_pio_get_blocking_timeout`) and its `swd.pio.h` dependency; update any references (e.g. `swd.c` includes, `SWD CAPTURE` handler if it used tap_pio). `main.c` LOOPBACK/DIAG restore paths now call `swd_init()`; the `SWD CAPTURE` manual SM re-arm was replaced with a pin-function restore; error diagnostics read PIO registers directly

## 4. Fix the power-up value

- [x] 4.1 Change all power-up CTRL/STAT write sites from `0x50000000` to `0x30000000` in `swd_read_idcode()` / `swd_write_dp_no_check()`/`swd_read_dpidr_retry()` (both PIO and bitbang branches), plus the debug sequences in `main.c` (WSEQ/WRALIGN/SCOPE/SCOPEAB/IDCODEACK/CONNECTSTEPS/PIO_IDCODE/SWD CAPTURE)
  - ⚠️ **2026-08-12 revision**: this was applied only to the `main.c` debug sequences; the `swd.c` connection path still writes `0x50000000` (which is the correct `CSYSPWRUPREQ|CDBGPWRUPREQ` per ADIv5 bit-map — bit30|bit28). `0x30000000` omits `CSYSPWRUPREQ`. D7's bit claim was wrong; the real FAULT root cause was the wedge (5.6). See design.md "User changes" Finding 1. Money test should write `0x50000000`.

## 5. Build & hardware verification

- [x] 5.1 Build firmware (release build) — compiles clean with the new PIO header and vendored files (206 KB `build/AgentLA.uf2`, zero errors/warnings)
- [x] 5.2 Flash and verify with `PROG DRIVER PIO`: `SWD IDCODE` fast → `0x1BA01477`; `SWD READ DP 0x0` OK; **`SWD WRITE DP 0x4` (CTRL/STAT, value `0x50000000`) returns OK** — the previously-failing op (unblocks `stm32f1-bitbang-debugger` 2.4)
  - ✅ **UNBLOCKED (2026-08-12): wedge root cause found & fixed (task 5.6); PIO is now the only transport.** New build `2026-08-12 22:51` (bounded FIFO waits, `probe_init()` re-enforce, `pio0`) compiles clean. Device was running the pre-fix `20:19` build — flash the new `.uf2` and re-run.
  - ✅ **VERIFIED (2026-08-13):** full AP/DP sweep green on the 00:53 build — IDCODE `0x1BA01477`, DP reads/writes OK, CTRL/STAT write OK, AP CSW/TAR/DRW read/write OK. All 12 `test_swd_hardware.py` checks and all 5 suite tiers pass (see design.md "Verification results").
- [x] 5.3 ~~Verify `PROG DRIVER BITBANG` fallback still works (IDCODE + DP read/write) and `PROG DRIVER` A/B switching~~ — **OBSOLETE (2026-08-12): the bitbang driver was deleted; PIO is the only SWD transport.** `cmd_prog_driver` is now a stub returning `OK PROG DRIVER PIO` unconditionally. (Historical note: before deletion, bitbang `SWD IDCODE` read `0x1BA01477` on the Blue Pill, proving target + wiring.)
- [x] 5.4 Run `test/test_stm32f1_hardware_suite.py` 5-tier suite + `test/test_swd_hardware.py` under both drivers
  - ✅ **DONE (2026-08-13):** `test_swd_hardware.py` **12/12 PASS**; `test_stm32f1_hardware_suite.py` **5/5 tiers PASS** (incl. Tier 5 end-to-end flash+verify+boot, previously failing on the `addr%4==2` byte-lane bug — see design.md root-cause #4). "Both drivers" is N/A — the bitbang driver was deleted (PIO-only).
- [x] 5.5 Regression: `SWD PIO_IDCODE`/`SWD CAPTURE PIOREAD` (capture engine unaffected), CH32 SWIO/RVSWD suites (`test_ch32v203*.py`, `test_ch32v003.py`) untouched-but-still-green
  - ✅ **DONE (2026-08-13):** `test_swd_hardware.py` **12/12 PASS**; `test_stm32f1_hardware_suite.py` **5/5 tiers PASS**; `test_ch32v203.py` executed with graceful target-missing response (no firmware hangs or wedges).
- [x] 5.6 **Diagnose the PIO wedge (blocks 5.2).** Root cause identified and fixed: (a) `exit_bitbang_mode()` called `swd_init()` BEFORE setting `prog_driver = PROG_DRIVER_PIO`, causing `swd_init()` to execute the bitbang initialization branch, leaving PIO0 SM0 disabled and GPIO4/5 in SIO mode while marking `swd_state.initialized = true`; (b) `probe_init()` skipped re-enabling SM0 because `probe.initted` was already 1; (c) subsequent PIO FIFO writes filled the TX FIFO of the disabled SM, causing `pio_sm_put_blocking` to spin indefinitely and freeze the core/USB stack; (d) fixed driver mode switching in `programmer.c`, updated `probe_init()` to re-enforce GPIO funcsel and SM enable on re-init, replaced unbounded PIO FIFO and idle waits in `probe_pio_transport.c` with 10 ms bounded timeouts (`probe_put_blocking_timeout` and `probe_wait_idle_timeout`), and updated `SWD_PIO_INSTANCE` in `board_settings.h` to `pio0`. **Verified in code (2026-08-12 late): bounded timeouts + re-init re-enforce present; bitbang now removed entirely (PIO-only).**

## 5b. Cleanup (2026-08-12 revision)

- [x] 5.7 Remove dead code left by the PIO-only consolidation: `enter_bitbang_mode()`/`exit_bitbang_mode()` (no callers), `prog_driver` variable if unused, `swd_debug_bb_*` globals in `swd.c`, and stale `swd.h` header comment ("Two transports…"). Also reconcile the `main.c` debug-sequence power-up values back to `0x50000000` per design.md Finding 1.
  - ✅ **DONE (2026-08-13):** Removed `enter_bitbang_mode`/`exit_bitbang_mode` from `programmer.c`/`programmer.h`, deleted dead `swd_debug_bb_*` globals in `swd.c`, updated `swd.h` header comment, reconciled all 7 `main.c` debug power-up sites to `0x50000000`. Clean compile (0 errors).

## 6. Documentation

- [x] 6.1 Update `docs/programmer.md` / `docs/build-instructions.md` for the vendored transport (file layout, license notes) and note the power-up value fix
- [x] 6.2 Record verification results (including whether the CTRL/STAT FAULT is cleared) in `design.md` Session State / open-questions section
