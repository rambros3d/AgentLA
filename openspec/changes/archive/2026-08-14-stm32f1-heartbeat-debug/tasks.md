## 1. Heartbeat Blink Firmware

- [x] 1.1 Update `test/bluepill_blink/src/main.cpp` `loop()` to output the double-pulse heartbeat rhythm: LOW 100 ms / HIGH 100 ms / LOW 100 ms / HIGH 700 ms (active-LOW PC13 LED)
- [x] 1.2 Build the heartbeat firmware with `pio run --project-dir test/bluepill_blink` and confirm `test/bluepill_blink/.pio/build/bluepill_f103c8/firmware.bin` is produced

## 2. CDC DEBUG Subcommands in AgentLA Firmware

- [x] 2.1 Add `cmd_debug` handler in `firmware/src/main.c` implementing `DEBUG HALT` (write `0xA05F0003` to `DHCSR` `0xE000EDF0`) and `DEBUG RUN` (write `0xA05F0000`)
- [x] 2.2 Implement `DEBUG STEP` in `cmd_debug` using `C_MASKINTS` (bit 18) and `C_STEP` (bit 2) in `DHCSR` for interrupt-safe single-stepping
- [x] 2.3 Implement `DEBUG REGS` in `cmd_debug` reading R0-R15 and xPSR via `DCRSR` (`0xE000EDF4`) / `DCRDR` (`0xE000EDF8`)
- [x] 2.4 Implement `DEBUG READ <addr>` and `DEBUG WRITE <addr> <val>` in `cmd_debug` for 32-bit target memory access while halted
- [x] 2.5 Register the `DEBUG` command in the `process_command` dispatch loop in `firmware/src/main.c`

## 3. Automated Heartbeat & Debug Verification Suite

- [x] 3.1 Create `test/test_stm32f1_heartbeat_debug.py` test harness script
- [x] 3.2 Implement Stage 1-2: build heartbeat firmware via PlatformIO and program `firmware.bin` via SWD FPEC flash (erase/write/verify at `0x08000000`)
- [x] 3.3 Implement Stage 3: issue `DEBUG HALT`, `DEBUG STEP`, `DEBUG REGS`, and `DEBUG WRITE`/`DEBUG READ` memory mutation over CDC and assert probe responses
- [x] 3.4 Implement Stage 4: resume the CPU with `DEBUG RUN` and sample PC13 LED output on `GPIO22` (L_CH2) with the logic analyzer, asserting the 100/100/100/700 ms double-pulse pattern
  - Implemented; **blocked on hardware** until the target-reset gap + capture hang below are resolved.
- [ ] 3.5 Run `python3 test/test_stm32f1_heartbeat_debug.py /dev/ttyACM0` against the connected Blue Pill and confirm all stages pass
  - ⛔ **BLOCKED** on two issues (see section 4): (a) `DEBUG RESET` does not reliably reboot the CPU (nRST effectiveness unverified), (b) LA `CAPTURE` hangs after `CONFIGURE` on the current build.

## 4. Session State & Handoff (2026-08-13)

### Test-harness fixes (applied to `test/test_stm32f1_heartbeat_debug.py`)

1. **`send_cmd()` always waited the full timeout** — 39 writes + 39 verifies x 5.3 s ≈ 414 s > the 400 s test limit, so the test appeared to "hang" mid-verify. The probe itself responds in ~50 ms; this was a host-side harness bug. Fixed: early-exit on terminal OK/ERR line + short idle grace.
2. **`DEBUG REGS` OK-first ordering** — the probe streams `OK DEBUG REGS` FIRST, then the 17 register lines. The early-exit initially truncated the registers; fixed with the idle-grace window.
3. **Memory-mutation parse bug** — `int(l.split('0x', 2)[1].split('=')[0], 16)` parsed the **address** (0x20000000) instead of the read-back **value**, so the test reported a mismatch even though the probe wrote and read back the marker correctly. Fixed: parse the value after `=`.

Result: stages 1–3 (connect, flash+verify, HALT/STEP/REGS/memory mutation) now pass.

### New firmware command: `DEBUG RESET`

Added to `cmd_debug()` in `firmware/src/main.c` — pulses the target nRST line (GPIO6, 20 ms low), releases, runs an SWD line reset + full `target_detect()` re-connect, then waits 100 ms for boot. Responds `OK DEBUG RESET` and the SWD link re-connects afterwards.

### Root cause of the stage 4/5 failure (target-reset gap)

The test flashes the firmware but **never resets the target**: it erases/writes flash while the OLD firmware is still executing from that same flash → the CPU faults (stage-3 register dump shows PC=0xFFFFFFFE = lockup, garbage R7–R12). Stage 4's `DEBUG RUN` resumes dead code, so the LA never sees the PC13 double-pulse (captured line is mostly HIGH with single-sample glitches).

### Open blockers for 3.5

1. **`DEBUG RESET` does not appear to actually reboot the CPU** — after reset + HALT the register dump still shows the same lockup values (PC=0xFFFFFFFE). Suspects: nRST wiring (GPIO6↔BluePill nRST) not verified, or the post-reset reconnect sequence. Also `DEBUG HALT` became nondeterministic after reset (OK once, ERR once).
2. **LA `CAPTURE` hangs after `CONFIGURE`** — `CONFIGURE channels=22 rate=100000` → `CAPTURE samples=300000 mode=RAW` prints `CAPTURING...` then never completes (`IsCapturing()` stuck). Worked on the previous firmware build; the only change since is the isolated `DEBUG RESET` handler, so this is either pre-existing flakiness or an interaction with the capture engine — needs isolation (fresh reflash, sweep channel/rate/samples, check PIO SM state).
