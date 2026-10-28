## Context

The `stm32f1-bitbang-debugger` change (tasks 1.1–2.3) built a working **bitbang** SWD transport and proved it on the STM32F103 Blue Pill — but task 2.4 is blocked by a deterministic FAULT ACK on the first CTRL/STAT write after a fresh connect. The PIO path (MioLink-derived modular architecture: 7 origin-addressed sub-programs in `firmware/pio/swd.pio`, dispatched by `tap_pio.c` with autopull/autopush and a C-tracked direction cache) has never been reliable.

A hardware sanity check settled the question of *where* the bug lives: Raspberry Pi's official **debugprobe** firmware (CMSIS-DAP, PIO-based SWD) was built for this same Pico 2 probe and connected to the same Blue Pill with pyOCD — full standard ADIv5 sequence (including the CTRL/STAT power-up write from a fresh state), 13/13 debugger feature sweep, heartbeat firmware flashed and executed. The target, wiring, and standard sequence are all proven fine; our in-house PIO/write-path mechanics are the suspect.

This change replaces the fragile modular PIO transport with debugprobe's proven one, vendored under permissive licenses (MIT for `probe.pio`/`probe.c`, Apache-2.0 for `sw_dp_pio.c`), both GPL-3.0-compatible.

## Goals / Non-Goals

**Goals:**
- Give the PIO driver branch of `swd_transfer()` the exact proven transaction shape (continuous-clock turnaround, data-phase back-off, protocol-error re-sync).
- Vendor only the transport core — no FreeRTOS, no CMSIS-DAP USB stack, no `DAP_config`.
- Keep the public SWD API (`swd_transfer`, `swd_read_dp/write_dp/read_ap/write_ap`, `swd_read_idcode`) and the bitbang fallback (`PROG DRIVER BITBANG`) intact.
- Delete the modular sub-program architecture and its filler-word/direction-cache workarounds.
- Fix the power-up value to `0x30000000` (bit28|bit29) in both branches.

**Non-Goals:**
- Porting `SWJ_Sequence`/`SWD_Sequence` (line reset + JTAG-to-SWD switch in `swd.c` are already wire-proven by scope capture).
- Touching the CH32 SWIO (`swio.pio`) or RVSWD (`rvswd.pio`) paths — separate PIO programs, unaffected.
- Replacing the bitbang driver; it remains the default reference path and fallback.
- Changing the host protocol, CLI commands, or Python SDK.

## Decisions

### D1. Port `SWD_Transfer` wholesale (Option B), not just the bit primitives (Option A)

- **Chosen**: Port `SWD_Transfer` from `references/debugprobe/src/sw_dp_pio.c` as the PIO transaction engine. `swd_transfer()` decodes our wire request byte into debugprobe's request layout (`APnDP|RnW<<1|A[2:3]<<2`) and calls it.
- **Alternative rejected (A — primitives only)**: vendor `probe_write_bits`/`probe_read_bits`/`probe_hiz_clocks` but keep our own transaction sequence. Rationale: our bitbang already has our sequence shape and produces the FAULT; if the bug lives in the *shape* (WAIT/FAULT data-phase handling, idle cycles, turnaround accounting) rather than the bits, A faithfully reproduces it. B inherits the exact behavior proven on this board.
- **Alternative rejected**: vendor the entire debugprobe firmware. Rationale: FreeRTOS + CMSIS-DAP USB would conflict with AgentLA's TinyUSB/analyzer architecture; only the transport is needed.

### D2. Extract `SWD_Transfer` into a standalone function with a config shim

- **Chosen**: Replace `DAP_Data.swd_conf.turnaround` / `DAP_Data.transfer.idle_cycles` / `DAP_Data.clock_delay` / `DAP_Data.timestamp` references with a small struct (`swd_pio_conf_t { uint8_t turnaround; uint8_t idle_cycles; }`) plus a direct `probe_set_swclk_freq()` call. Drop the timestamp capture (unused by AgentLA).
- **Rationale**: kills the CMSIS-DAP dependency without behavioral change; the constants are config we already own (`swd_state.turnaround_cycles = 1`, `swd_state.idle_cycles = 8`).
- Parity verification (`__builtin_popcount`) and the WAIT/FAULT dummy-phase back-off are ported verbatim.

### D3. Keep the wire-byte `swd_transfer()` API

- **Chosen**: `swd_transfer(request_byte, *data)` keeps its signature; the PIO branch decodes the byte (2 lines) and calls `SWD_Transfer`. All callers (`main.c`, `target.c`, `cmsis_dap.c`, `ch32.c`) are untouched.
- **Rationale**: the request byte layout (start/parity/stop/park) was validated by wire capture; only the transport behind it changes.

### D4. Vendor layout and build integration

- **Chosen**: new `firmware/src/programmer/debugprobe/` containing `probe.pio` (renamed to `swd_probe.pio` to avoid CMake header collision with `probe.h`), `probe_pio_transport.c` (the `probe_*` primitives, adapted: pins from `swd.h`, `CPU_CLOCK` define), and `swd_transfer_pio.c` (the ported `SWD_Transfer`). `CMakeLists.txt` gains a `pico_generate_pio_header` for the new PIO file. License headers kept verbatim; a comment notes the vendored upstream commit (`3fff5b2`) and local adaptations.
- **Rationale**: keeps vendored code isolated and diffable against upstream; avoids touching `probe.c`'s includes (`probe_config.h`, `DAP_config.h`) which don't exist in AgentLA.

### D5. Delete the modular PIO architecture

- **Chosen**: remove the 7 sub-programs and `SWD_ORIGIN_*` constants from `firmware/pio/swd.pio`, and `tap_pio.c`'s origin-stream/DMA dispatch (`tap_pio_dma_send_uint32`, `tap_pio_dma_send_recv_uint32`). `current_swdio_dir` tracking is removed; direction is now in-band in the FIFO command stream.
- **Rationale**: this is the code under suspicion (autopull filler words, stale direction cache, `pio_sm_restart` recovery). Keeping it alongside the new transport doubles maintenance without benefit. `tap_pio.c`'s `swd_diag_state` diagnostics are dropped with it (the `SWD CAPTURE`/`SCOPE` diagnostics in `main.c` remain).

### D6. PIO init and clock

- **Chosen**: single self-contained program via `pio_add_program()` (no `_at_offset`, no origin bookkeeping). `probe_set_swclk_freq(1000)` at init; `swd_set_clock()` maps kHz → the same divider math. Pins stay PIO0 SM0 on GPIO4/5; `PROBE_PIN_*` macros are `#define`d from `swd.h` values.
- **Rationale**: the vendored `probe_sm_init` shape (side-set SWCLK, out/in/set SWDIO, autopull/autopush **off**) is the entire point — explicit pull/push, no phase coupling.

### D7. Power-up value

- **Chosen**: `0x30000000` (CDBGPWRUPREQ|CSYSPWRUPREQ) replaces `0x50000000` at the `swd_read_idcode()` / `swd_write_dp_no_check()` call sites in both branches. Confirmed working by pyOCD during the sanity check; the FAULT was value-independent, so this is a correctness fix, not the FAULT trigger.

## Risks / Trade-offs

- **Sequence-shape regression on non-STM32 targets** → The CH32/RV transports are separate PIO programs and untouched; the STM32F1 SW-DP demonstrably accepts this exact sequence (13/13 sweep). Verify with the existing STM32F1 hardware suite.
- **`probe_read_bits` blocks** (`pio_sm_get_blocking`) → Fine in the single-threaded command loop; the `PROG_MODE_IS_ACTIVE()` guard already prevents programmer/SWD and capture-engine overlap on PIO0.
- **PIO0/SM0 contention with the capture engine** → Same guard as today; capture uses other SMs and disables programmer mode when active.
- **Vendored-code drift from upstream** → Pin to commit `3fff5b2` in the header; local adaptations limited to pins/CPU_CLOCK/config-shim.
- **Clock semantics differ** (`probe_set_swclk_freq` derives from `CPU_CLOCK`) → define `CPU_CLOCK` to the configured sys_clk (125 MHz default; same value used by `tap_pio_set_sm_freq` today).
- **License compliance** → MIT + Apache-2.0 headers retained verbatim in vendored files; compatible with GPL-3.0 (proposal documents this).
- **Bitbang stays the default; PIO may get less real-world exercise** → `SWD PIO_IDCODE` and the STM32F1 suite's PIO tiers keep it covered; `PROG DRIVER PIO` enables A/B on demand.

## Migration Plan

1. Vendor transport files + CMake integration (new files; build stays green because nothing is wired yet).
2. Port `SWD_Transfer` and wire the PIO branch of `swd_transfer()`; bitbang branch untouched.
3. Delete the modular sub-program architecture and `tap_pio.c` dispatch.
4. Fix power-up value in both branches.
5. Hardware verification: `SWD IDCODE`, `SWD READ DP 0x0`, `SWD WRITE DP 0x4` (CTRL/STAT — the previously failing op), then the STM32F1 suite; rollback = `PROG DRIVER BITBANG` (no firmware change needed).

## Hardware verification (2026-08-12) — PIO transport WEDGES the firmware

**Status: task 5.2 blocked on a new defect discovered during first PIO use.**

### What was verified

- AgentLA flashed to the Pico 2 probe via BOOTSEL (`build/AgentLA.uf2`, build `2026-08-12 20:19` git `e5b50be`). Device enumerates cleanly: `1209:414d`, 5 interfaces (CDC0 CLI + CDC1 debug + HID CMSIS-DAP), strings readable, ttyACM0/ttyACM1/hidraw all bound.
- **Bitbang path works on hardware**: `SWD IDCODE` under `PROG DRIVER BITBANG` returns `OK SWD IDCODE 0x1BA01477` — the Blue Pill target is connected, wiring is good, and our bitbang transport reads IDCODE correctly.
- **`PROG DRIVER PIO` A/B switch itself is fine** (command succeeds, device stays alive).

### The defect

The **first `SWD IDCODE` under `PROG DRIVER PIO` hangs the entire firmware** (task 5.2's money test cannot even be attempted):

1. Device USB control pipe dies — `GET_DESCRIPTOR` times out at 3 s, `open()` on ttyACM0 blocks forever, pyOCD cannot see the probe.
2. The USB address stays constant and the device does **not** re-enumerate — so it is **not** a hardfault→watchdog reboot (the hardfault handler calls `watchdog_reboot`). It is a **soft busy-loop** in the main thread: once wedged, `tud_task()` is never called again, so all control transfers stall.
3. **Trigger is precisely the PIO path**, not the port open: a staged probe showed raw open, pyserial open, `VERSION`, and `PROG DRIVER` all keep the device ALIVE; `SWD IDCODE` under bitbang keeps it ALIVE; switching to PIO and running `SWD IDCODE` wedges it instantly.
4. A physical replug revives the device (host + firmware restart); it re-wedges on the same first-PIO-use. (An earlier "wedge at first contact" observation before any replug was the same defect — the first interaction in that session was `PROG DRIVER PIO` + `SWD IDCODE`.)

### Root-cause analysis so far

- The vendored PIO program is **byte-identical** to the proven debugprobe build: `build/swd_probe.pio.h` vs `references/debugprobe/build-pico2/probe.pio.h` — same 11 instruction words, same offsets (`write_cmd 0`, `get_next_cmd 3`, `read_cmd 8`), same wrap (3→10), same sideset (`2, opt`), same `PICO_PIO_VERSION=1`. The program is not the problem.
- The C driver (`probe_pio_transport.c`) is a faithful port of `probe.c`. The only unbounded waits in the PIO path are `pio_sm_put_blocking()` (spins if the TX FIFO fills because the SM is not consuming) and `probe_wait_idle()` (spins on TXSTALL). `probe_read_bits()` is bounded (10 ms).
- Prime suspect: the SM is **not consuming the command FIFO** after `probe_init()` (clock divider not applied / SM disabled / `pio_sm_exec` jump landed wrong / GPIO function conflict on GPIO4-5), so the first `probe_write_bits(8, prq)` fills the FIFO and `pio_sm_put_blocking` spins forever.
- A stale inconsistency was also noted: `swd.h` now says `SWD_PIO pio0` / `SWD_SM 0`, but `board_settings.h` still has `SWD_PIO_INSTANCE pio1` (leftover from the modular era) — need to confirm nothing else initializes PIO0 SM0 or GPIO4/5 before/around `probe_init()` (adapter UART lazy-init on GPIO4/5 is a candidate; `adapter_init_i2c_only()` is called at boot).

### Root cause identified and resolved (2026-08-12, Task 5.6 Complete)

- **Root Cause**: `exit_bitbang_mode()` in `firmware/src/programmer/programmer.c` was calling `swd_init()` BEFORE setting `prog_driver = PROG_DRIVER_PIO`. Because `prog_driver` was still `PROG_DRIVER_BITBANG`, `swd_init()` executed the bitbang initialization branch (`swd_bb_gpio_init()`), which configured GPIO4 and GPIO5 for SIO (bitbang) and left PIO0 SM0 uninitialized and disabled while setting `swd_state.initialized = true`.
- **Secondary Factor**: `probe_init()` in `probe_pio_transport.c` skipped re-enabling SM0 or re-configuring GPIO funcsel when called again because `probe.initted` was already 1.
- **Symptom Mechanism**: When `SWD IDCODE` ran under `PROG_DRIVER_PIO`, `probe_write_bits` pushed command words into the 4-entry TX FIFO of the disabled SM0. Since SM0 was disabled and GPIOs were in SIO mode, the SM never consumed the words. The 4th push invoked `pio_sm_put_blocking()`, which spun in an infinite loop (`while (pio_sm_is_tx_fifo_full)`), freezing core 0 and preventing `tud_task()` from executing (soft hang/wedge).
- **Fixes Applied**:
  1. Updated `exit_bitbang_mode()` and `enter_bitbang_mode()` in `programmer.c` to explicitly set `prog_driver = PROG_DRIVER_PIO` prior to calling `swd_deinit()` / `swd_init()`.
  2. Updated `probe_init()` so that re-initialization calls re-enforce `probe_gpio_init()` (restoring GPIO4/5 function select to `GPIO_FUNC_PIO0`), clear FIFOs, reset `fdebug`, and re-enable SM0.
  3. Replaced all infinite FIFO writes (`pio_sm_put_blocking`) and idle loops (`probe_wait_idle`) in `probe_pio_transport.c` with 10 ms bounded timeout loops (`probe_put_blocking_timeout` and `probe_wait_idle_timeout`), guaranteeing the firmware will never hang even if a PIO FIFO stalls.
  4. Updated `SWD_PIO_INSTANCE` in `board_settings.h` to `pio0`.

## User changes (2026-08-12, late session) — PIO-only transport; bitbang removed

A new round of changes was made after task 5.6. Analysis of `git diff` vs. the documented plan:

### Changes applied

1. **Bitbang driver deleted entirely**: all `swd_bb_*` functions removed from `swd.c`/`swd.h` (`swd_bb_gpio_init`, `swd_bb_out_bits`, `swd_bb_in_bits`, `swd_bb_line_reset_ext`, `swd_bb_jtag_to_swd`, `swd_bb_transfer`, `swd_bitbang_read_idcode`, `swd_bb_write_turn_clocks`). `swd_get_idcode_with_fallback()` in `main.c` is now just `swd_read_idcode()` — no A/B fallback. `prog_driver` now defaults to `PROG_DRIVER_PIO`, `cmd_prog_driver` is a stub returning `OK PROG DRIVER PIO` unconditionally, and `enter_bitbang_mode()`/`exit_bitbang_mode()` in `programmer.c` are dead code (no callers remain).
2. **Modular PIO architecture fully removed**: `firmware/pio/swd.pio` and `tap_pio.c`/`tap_pio.h` deleted; `CMakeLists.txt` now generates the PIO header from `firmware/src/programmer/debugprobe/swd_probe.pio`. `swd.h` no longer includes `swd.pio.h`.
3. **Wedge fix confirmed in code** (task 5.6 implementation): `probe_pio_transport.c` now uses bounded `probe_put_blocking_timeout()` (10 ms) and a bounded `probe_wait_idle()` (10 ms on TXSTALL) instead of unbounded `pio_sm_put_blocking`; `probe_init()` re-enforces GPIO funcsel + SM enable on re-init; `SWD_PIO_INSTANCE` → `pio0`.
4. **`stm32f1_flash_read()` rewritten** in `target.c`: correct AHB-AP read pipelining (write CSW/TAR once, dummy `AP_DRW` read to start the bus cycle, then `AP_DRW` reads with a final `DP_RDBUFF` for the last word).
5. **Tests updated**: `test_blink_bluepill.py` + `test_stm32f1_hardware_suite.py` now send `PROG DRIVER PIO` first and use `DEBUG READ/HALT/WRITE`/`DEBUG WRITE 0xE000ED0C 0x05FA0004` (AIRCR reset) instead of raw `SWD WRITE AP` sequences.
6. **`references/blackmagic/` vendored tree deleted** (~100 K lines) — cleanup; provenance comments remain in `swd.c`.

### Findings

1. **Power-up value inconsistency + bit-map error in D7.** `swd.c` connection path (`swd_read_idcode()` / `swd_read_dpidr_retry()`) still writes `0x50000000`, while the `main.c` debug sequences (WSEQ/IDCODEACK/CONNECTSTEPS/SCOPEAB/PIO_IDCODE/SWD CAPTURE) now write `0x30000000`. ADIv5 CTRL/STAT layout is: bit31 `CSYSPWRUPACK`, bit30 `CSYSPWRUPREQ`, bit29 `CDBGPWRUPACK`, bit28 `CDBGPWRUPREQ`. The canonical "power everything up" request is **`0x50000000` = CSYSPWRUPREQ | CDBGPWRUPREQ**. `0x30000000` sets only `CDBGPWRUPREQ` (plus the read-only ACK bit 29) — it omits `CSYSPWRUPREQ`. D7's claim "0x30000000 (CDBGPWRUPREQ|CSYSPWRUPREQ)" is therefore incorrect, and the main.c debug sequences request debug-power only. Since the real connection path (`swd.c`) uses `0x50000000` and the bitbang hardware run passed with it, the "power-up value fix" (task 4) was a red herring for the CTRL/STAT FAULT — the actual root cause was the wedge (task 5.6). **Recommendation: keep `0x50000000` in `swd.c`; reconcile the main.c debug sequences to `0x50000000` so the money test (`SWD WRITE DP 0x4`) exercises the real power-up request.**
2. **Scope change vs. the original Non-Goals.** This change originally stated "Replacing the bitbang driver … it remains the default reference path and fallback" as a non-goal. The bitbang driver is now deleted and PIO is the only transport — a deliberate consolidation, but the plan must be updated (task 5.3 obsolete; `PROG DRIVER` A/B switching no longer exists).
3. **Leftover dead code**: `swd_debug_bb_*` globals in `swd.c` are no longer referenced by `main.c` (externs removed) — cleanup candidates. `current_swdio_dir` is now vestigial in the PIO-only path (still used to reset state at init/read_idcode).
4. **`swd.h` comment header** still describes "Two transports, selected at runtime" — stale after bitbang removal.

## Open Questions

- Whether the ported transport independently clears the CTRL/STAT FAULT is the acceptance test for this change (task 2.4 unblock). With the wedge fixed (task 5.6) and the PIO-only consolidation, hardware verification (tasks 5.2, 5.4, 5.5) can proceed. The money test should write `0x50000000` (see Finding 1).
- Whether to reconcile the main.c debug sequences back to `0x50000000` (recommended) or keep `0x30000000` deliberately — resolve while flashing/testing.
- Whether to clean up the dead `enter_bitbang_mode`/`exit_bitbang_mode`, `prog_driver`, and `swd_debug_bb_*` leftovers (task 5.7 candidate).

## Root-cause chain: "why did debugprobe work and ours doesn't" (2026-08-13)

With the vendored debugprobe PIO transport in place and connecting fine (`SWD IDCODE` → `0x1BA01477`), four independent bugs were found and fixed, each proven by a discriminating on-target experiment before the fix:

### 1. AP register map: `AP_DRW` was `0x8` (reserved) instead of `0x0C`

`swd.h` defined `AP_DRW 0x8`. Per ADIv5 §A3.4.1 the AHB-AP DRW register is at **`0x0C`**; `0x08` is reserved. Every memory/flash read and write therefore hit a reserved register, which reads back `0x00000000` and ignores writes. This was invisible to the old DP-only tests (they never touched AP memory) and is why "SRAM always reads zero" predated the transport swap. pyOCD's authoritative `ahb_ap.py` confirms `MEM_AP_DRW = 0x0C`. **Fix:** `AP_DRW 0xC` (comment updated; bogus `AP_MB` removed).

### 2. Posted AP reads: single-shot `swd_read_ap()` returns stale data

ADIv5 AP reads are **posted**: the data phase of an AP read returns the *previous* AP read's result; the just-posted value is latched into DP RDBUFF (0x0C). debugprobe is a dumb CMSIS-DAP transport — it returns raw data phases and lets the *host* (pyOCD/OpenOCD) post-then-collect via RDBUFF. Our firmware's own debugger called `swd_read_ap()` as a single shot, so every AP read returned the previous transaction's data (pipeline test: `R CSW` → stale, `R TAR` → CSW's value). **Fix (central, in `swd_read_ap()`):** post the AP read, then read DP RDBUFF and return that. All callers (`main.c`, `target.c`, `cmsis_dap.c`) benefit; the double-`AP_DRW`-read workarounds at some call sites were simplified.

### 3. AP FAULT desync: hardcoded `data_phase=true` corrupts ADIv5.0 links

Reading `DBGMCU_IDCODE` (0x40015800) on this Blue Pill (a non-ST clone — the peripheral doesn't decode, so the AHB bus returns an error) **deterministically FAULTs** the AP. After the FAULT, the ported engine consumed a hardcoded 33-bit dummy data phase (`conf.data_phase = true`). The STM32F1 SWJ-DP is an ADIv5.0 DP which **does not drive a data phase on WAIT/FAULT** — consuming those 33 clocks fed the target's line state machine arbitrary high-Z bits which it misparsed as a new request → nondeterministic desync that even a bare line reset didn't clear (only a full JTAG-to-SWD re-connect recovered). debugprobe leaves this decision to the host (CMSIS-DAP `DAP_SWD_Configure` `data_phase` bit); our port hardcoded it. **Fixes:** (a) `conf.data_phase = false` in `swd_pio_transfer_wire()` (verified: session survives the DBGMCU FAULT, 3/3 rounds); (b) `swd_recover_from_ack_error()` — on FAULT/protocol error run the proven full-link recovery (JTAG-to-SWD + DPIDR + ABORT `0x1E` + power-up `0x50000000` + poll), same sequence `swd_read_idcode()` uses; (c) `swd_write_ap`/`swd_read_ap` retry once after recovery (the first post-fault op returns the latched FAULT; recovery clears it and the retry succeeds).

### 4. Flash 16-bit byte-lane positioning (Tier 5 failure)

`test_stm32f1_hardware_suite.py` Tier 5 (end-to-end flash + verify) failed with `mismatch at offset 0x0003: expected 0x20 got 0x00`. A value-sweep experiment isolated the discriminator to the **address**: every halfword programmed to an `addr%4==2` address (addr+2, +6, +A…) read back `0x0000`, while 4-aligned halfwords succeeded regardless of value. An SRAM lane experiment (write `0xBBBB` to `0x20000002` with data right-aligned vs. left-shifted) proved the STM32F1 AHB-AP **does not right-align sub-word data** — the halfword must be pre-positioned in the DRW byte lane selected by `addr bit[1]` (`bits [31:16]` for `addr%4==2`, `[15:0]` for `addr%4==0`). BMP's `adiv5_pack_data()` (git HEAD, `references/blackmagic/src/target/adiv5.c`) does exactly this: `*data = (uint32_t)value << (8U * (dest & 2U))`. Our `stm32f1_flash_write()` always right-aligned → halfwords at `addr%4==2` programmed as `0x0000` (and since STM32F1 flash can't be reprogrammed, the verify caught it permanently). **Fix:** `swd_write_ap(AP_DRW, (uint32_t)halfword << (8U * ((addr + i) & 2U)))`.

### Why debugprobe never hit any of these

- Bug 1: debugprobe never interprets AP registers — the host sends raw AP register addresses and data; there is no hardcoded `AP_DRW` in the firmware.
- Bug 2: debugprobe is a dumb transport — the host performs post-then-RDBUFF, so the posted-read idiom lives host-side (pyOCD), not in probe firmware.
- Bug 3: `data_phase` is a host-controlled CMSIS-DAP configuration bit; the probe never decides it.
- Bug 4: debugprobe doesn't program flash — the host writes bytes through `DAP_Transfer` and the MEM-AP packs sub-word data host-side.

All four bugs lived in AgentLA's *firmware-resident debugger* layer (register maps + AP access idioms + flash algorithm), not in the PIO wire transport — which is exactly the layer that a dumb CMSIS-DAP probe doesn't have.

### Verification results (2026-08-13, Blue Pill + new build 00:53)

- `test/test_swd_hardware.py`: **12/12 PASS** (SWD PINS, IDCODE, TEST, RESET, DPIDR, TARGET_DETECTION, CTRL/STAT, RDBUFF, AP CSW write/read, FLASH_COMMANDS)
- `test/test_stm32f1_hardware_suite.py`: **5/5 tiers PASS** — including Tier 5 end-to-end: erase → write 9920-byte blink firmware (39 chunks) → verify → reset → app runs
- Flash round-trip: `FLASH WRITE 00500020AD1E0008` → `FLASH READ` returns `00,50,00,20,AD,1E,00,08`; word reads confirm `0x20005000` (initial SP) and `0x08001EAD` (reset vector)
- AP read pipeline: first read returns fresh CSW/TAR values (posted-read fix); SRAM `DEBUG WRITE`/`DEBUG READ` round-trips; session survives the DBGMCU FAULT with the recovery + retry

## Session state (2026-08-13, end of session) — where we are

### What was completed this session

1. **All four root-cause bugs fixed and hardware-verified** (see the root-cause chain above):
   - `AP_DRW 0x8 → 0x0C` in `swd.h` (register map)
   - `swd_read_ap()` posted-read compensation via DP RDBUFF (central fix)
   - `data_phase=false` + `swd_recover_from_ack_error()` + retry-once in `swd_write_ap`/`swd_read_ap` (AP FAULT desync)
   - `stm32f1_flash_write()` 16-bit byte-lane shift `(uint32_t)halfword << (8U * ((addr + i) & 2U))` (Tier 5)
2. **Code-review cleanup applied**: removed the redundant pre-loop DP_SELECT block + duplicate `select`/`res` declarations in `swd_read_ap()` (the loop's own check handles DP_SELECT, including re-write after recovery invalidates the cache). Rebuilt, reflashed, re-verified flash round-trip byte-perfect (`00500020AD1E0008` → `00,50,00,20,AD,1E,00,08`).
3. **`test/test_swd_hardware.py`: 12/12 PASS; `test/test_stm32f1_hardware_suite.py`: 5/5 tiers PASS** (final build timestamp 00:53 + cleanup rebuild; Blue Pill probe flashed, boot counter BOOTS 7/3).
4. **Tasks 5.2, 5.4 marked complete** in `tasks.md`; OpenSpec `validate` 8/8 ✓.

### Remaining for the next session

- **Task 5.5 (open)**: regression sweep — `SWD PIO_IDCODE`/`SWD CAPTURE PIOREAD` (capture engine unaffected), CH32 SWIO/RVSWD suites (`test_ch32v203*.py`, `test_ch32v003.py`), `test_swd_idcode.py`/`test_swd_hardware.py` re-run. The capture engine shares PIO0 with the vendored SWD transport, so this is the one area the PIO swap could have affected.
- **Task 5.7 (open)**: dead-code cleanup — `enter_bitbang_mode()`/`exit_bitbang_mode()` (no callers), `prog_driver` variable if unused, `swd_debug_bb_*` globals in `swd.c`, stale `swd.h` header comment ("Two transports…"), and reconcile `main.c` debug-sequence power-up values to `0x50000000` (design.md Finding 1).
- **Archive**: after 5.5 + 5.7, archive this change (`openspec archive`).

### Carry-over notes for the next session

- The probe currently runs the cleanup rebuild (flash round-trip verified). Re-running the full 5-tier suite is cheap (~2 min) and confirms the final binary before archiving.
- `stm32f1_flash_read()` is already rewritten for correct AHB-AP pipelining (posted-read semantics now live centrally in `swd_read_ap()`; the rewrite predates that and is consistent with it).
- The `SWD PIO_IDCODE`/`SWD CAPTURE` commands in `main.c` still reference `NO_TRIGGER_CAPTURE_program` (capture.pio) — unaffected by the transport swap, but the 5.5 sweep should confirm no PIO0 SM contention with the vendored `swd_probe.pio` on SM0.

