## 1. Core Vector Catch & Dual-AP Implementation

- [ ] 1.1 Update `CortexM::connect_under_reset` in `crates/agentla-programmer/src/target/cortex_m.rs` to route low-power debug clocking via AP 0 (`0xE00E4004`) while under reset, arm vector catch, release reset, and select AP 1 (`AHB-AP`).
- [ ] 1.2 Run host unit tests in `crates/agentla-programmer` and `crates/agentla-targets` to verify zero regression across architectures.

## 2. Programmer Workflow Integration

- [ ] 2.1 Update `flash_target_port_global` in `firmware-rs/src/programmer/mod.rs` to invoke `connect_under_reset` on STM32H5 targets to ensure cycle-0 halt before flash algorithm execution.
- [ ] 2.2 Update `erase_flash_port_global` in `firmware-rs/src/programmer/mod.rs` to invoke `connect_under_reset` on STM32H5 targets.

## 3. Hardware Flashing & SWD Verification

- [ ] 3.1 Build AgentLA firmware and deploy to probe via OTA.
- [ ] 3.2 Flash `n20-servo.bin` to the STM32H503 target and confirm post-programming verification succeeds with HTTP 200.
- [ ] 3.3 Run `test/test_cmsis_dap.py` to verify full CMSIS-DAP suite passes.
- [ ] 3.4 Verify embedded GDB server connects and inspects target memory and registers.
