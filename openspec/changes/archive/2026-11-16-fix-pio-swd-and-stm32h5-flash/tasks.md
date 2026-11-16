## 1. PIO SWD Driver & DAP Fixes

- [x] 1.1 In `crates/agentla-programmer/src/pio/swd.rs`, change `set_sideset_count(2)` to `set_sideset_count(2)` (1 bit enable + 1 bit pin) and remove spurious turnaround clocks
- [x] 1.2 In `crates/agentla-programmer/src/pio/swd.rs`, enforce at least 2 trailing idle cycles (`max(self.idle_cycles, 2)`) on all SWD transfers in `transfer_raw`
- [x] 1.3 In `crates/agentla-programmer/src/dap.rs`, implement `MATCH_VALUE` (`0x10`) and `MATCH_MASK` (`0x20`) support in `handle_transfer` and align `DAP_TransferBlock`
- [x] 1.4 Clean up diagnostic intercept in `firmware-rs/src/programmer/mod.rs` `read_flash_port_internal`

## 2. STM32H5 Flash Driver Implementation

- [x] 2.1 In `crates/agentla-targets/src/engines/stm32.rs`, implement STM32H5 flash unlock routine for `FLASH_NSKEYR`
- [x] 2.2 Implement STM32H5 sector erase using `FLASH_NSCR` `SER` and `SNB`
- [x] 2.3 Implement STM32H5 128-bit quad-word programming loop with `FLASH_NSCR` `PG` and `FLASH_NSSR` busy polling
- [x] 2.4 Implement 1 KiB boundary chunking (`1024 - (addr & 1023)`) for TAR auto-increment and posted-read `DP_RDBUFF` termination
- [x] 2.5 Wire STM32H5 flash functions into `flash_target_port_global` in `firmware-rs/src/programmer/mod.rs`

## 3. Firmware Build & OTA Deployment

- [x] 3.1 Build release firmware (`cargo build --release -p firmware-rs --target thumbv8m.main-none-eabihf --features slot-b`)
- [x] 3.2 Package OTA binary using `tools/package_ota.py`
- [x] 3.3 Upload firmware to `http://10.10.10.10/api/v1/firmware/ota` and verify Slot B activation via `GET /api/v1/firmware/info`

## 4. Hardware Verification & Capabilities Test

- [x] 4.1 Verify CMSIS-DAP: Run `test/test_cmsis_dap.py` and confirm all 12 DAP checks pass with `ack=1`
- [x] 4.2 Verify GDB Server / probe-rs: Run `probe-rs info --chip STM32H503KBUx --probe 1209:414d` and confirm CoreSight AP topology discovery
- [x] 4.3 Verify Direct Flash / Host unit test suite: Run `cargo test -p agentla-programmer --target x86_64-unknown-linux-gnu` and `cargo test -p agentla-targets --target x86_64-unknown-linux-gnu` (all 30 tests pass)
