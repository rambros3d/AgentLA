## 1. Teardown primitive in `agentla-programmer`

- [ ] 1.1 Add `DetachPolicy` (`Leave`, `Halt`, `Run`) in `crates/agentla-programmer/src/target/cortex_m.rs`.
- [ ] 1.2 Add `CortexM::teardown(&mut self, policy: DetachPolicy) -> Result<(), CortexError>` that restores the snapshotted `DEMCR`, then applies the policy to `DHCSR`. No polling loops.
- [ ] 1.3 Capture `DEMCR` during `connect_under_reset` after AP/CSW selection and before arming; store it as `Option<u32>` on `CortexM`.
- [ ] 1.4 Capture `DEMCR` in `auto_configure_for_dpidr` / `attach` paths for non-reset sessions.
- [ ] 1.5 Export `DetachPolicy` from the crate root and re-export from `firmware-rs/src/programmer/target/cortex_m.rs`.

## 2. Session ownership in firmware

- [ ] 2.1 Add `profile`, `saved_demcr`, and `detach` fields to `SwdSession` in `firmware-rs/src/programmer/pins.rs`.
- [ ] 2.2 Add `SwdSession::open(mode)` and `SwdSession::set_detach_policy(policy)`.
- [ ] 2.3 Extend `Drop for SwdSession` to call teardown before `transport.deinit()` and `tristate_pins()`.
- [ ] 2.4 Confirm `release()` still returns `ProgrammerPins` unchanged and does not double-apply teardown.

## 3. Call-site migration

- [ ] 3.1 Migrate `flash_target_port_global` and `erase_flash_port_global` to `AttachMode::Auto` + `DetachPolicy::Run`; remove the `attach()`-then-`connect_under_reset()` double attach.
- [ ] 3.2 Migrate `read_flash_port_internal` to `AttachMode::Auto` + `DetachPolicy::Leave` (non-intrusive read preserved).
- [ ] 3.3 Migrate `detect_target_port` and `halt_target_port_global` to `Auto` + `Leave` / `Halt`.
- [ ] 3.4 Migrate `resume_target_port_global` to `Auto` + `Run`.
- [ ] 3.5 Migrate the reset-catch REST endpoint to `UnderReset` + `Halt`.
- [ ] 3.6 Migrate `firmware-rs/src/programmer/gdb_server.rs` to `Auto` + `Run` on disconnect.

## 4. Host tests

- [ ] 4.1 Assert `VC_CORERESET` is cleared after `teardown(Run)` using the mock transport.
- [ ] 4.2 Assert `TRCENA` survives teardown when it was set in the snapshot.
- [ ] 4.3 Assert `Halt` leaves `DHCSR.S_HALT` set and clears `VC_CORERESET`.
- [ ] 4.4 Assert a failed `mem_read32` during teardown is swallowed and pins are released.
- [ ] 4.5 Run `cargo test -p agentla-programmer` and `cargo test -p agentla-targets`.

## 5. Hardware verification

- [ ] 5.1 Build release firmware and OTA to the probe (`tools/package_ota.py`, slot B).
- [ ] 5.2 Flash `n20-servo.bin` to STM32H503 via `POST /api/v1/programmer/flash`.
- [ ] 5.3 Confirm `GET /api/v1/programmer/read?addr=0xE000EDFC` returns `0x00000000` after the flash.
- [ ] 5.4 Confirm BareBoot launches N20-Servo without a manual `DEMCR` write (no halt at `0x08000240`).
- [ ] 5.5 Confirm the I2C target answers at `0x40` once the application is running.

## 6. Bookkeeping

- [ ] 6.1 Run `openspec validate cortex-m-debug-session-lifecycle`.
- [ ] 6.2 Sync specs and archive the change.