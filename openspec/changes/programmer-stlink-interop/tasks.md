## 1. Reply-before-probe

- [ ] 1.1 In `crates/agentla-programmer/src/stlink.rs`, restructure the
  `DBG_APIV1_ENTER | DBG_APIV2_ENTER` arm so it sets `mode`, sets `idcode = 0`,
  writes `STLINK_OK`, and only then attempts a bounded DP read.
- [ ] 1.3 Cap the DP read to a single `read_dp` so it cannot enter the
  `read_dpidr_retry` multi-frequency, multi-sequence loop.

## 2. AP/CSW reset prologue

- [ ] 2.1 Add the hybridlink exempt-list at the top of `debug_command`, resetting
  `current_ap = 0` for every command except `RW_MISC_IN`, `RW_MISC_OUT`,
  `INIT_AP`, `CLOSE_AP_DBG`, `READ_DAP_REG`, `WRITE_DAP_REG`.

## 3. Transport error logging

- [ ] 3.1 In `firmware-rs/src/usb.rs`, replace `let _ = cmd_in.write(...)` in the
  ST-Link reply path with a logged error carrying the reply length.
- [ ] 3.2 Log failures from `cmd_out.read(...)` in the ST-Link command loop.
- [ ] 3.3 Confirm the CMSIS-DAP and WCH-Link tasks keep their existing
  behaviour (unchanged).

## 4. Host tests

- [ ] 4.1 Add a mock-transport test asserting ENTER_SWD returns 2 with
  `STLINK_OK` when `read_dp` fails.
- [ ] 4.2 Add a test asserting `current_ap` resets to 0 for `DBG_RUNCORE` after a
  command that set it to 1.
- [ ] 4.3 Run `cargo test -p agentla-programmer --target x86_64-unknown-linux-gnu`.

## 5. Hardware verification

- [ ] 5.1 Build release firmware and OTA to the probe.
- [ ] 5.2 Switch to ST-Link mode: `curl -X POST ".../api/programmer/mode?mode=stlink"`.
- [ ] 5.3 Run `st-info --probe` and record the reported chipid and flash size.
- [ ] 5.4 Run a read-only `st-flash` read of `0x08000000` and compare with
  `F8 7F 00 20 41 02 00 08`.
- [ ] 5.5 Confirm `python3 test/test_cmsis_dap.py` still reports 12/12.

## 6. Bookkeeping

- [ ] 6.1 Run `openspec validate programmer-stlink-interop`.
- [ ] 6.2 Sync specs and archive once hardware verification passes.