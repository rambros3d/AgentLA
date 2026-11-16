## 1. STM32H5 DBGMCU & Target Attach Configuration

- [x] 1.1 Implement `stm32h5_configure_dbgmcu` in `crates/agentla-targets/src/engines/stm32.rs` to configure `DBGMCU_CONFIG` (`DBG_STOP | DBG_STANDBY`) and freeze watchdogs via `DBGMCU_APB1LFREEZE`.
- [x] 1.2 Call DBGMCU configuration upon STM32H5 attachment in `firmware-rs/src/programmer/target/mod.rs` and `firmware-rs/src/programmer/mod.rs`.

## 2. Direct Flash & Non-Intrusive Memory Reading

- [x] 2.1 Update `read_flash_port_internal` in `firmware-rs/src/programmer/mod.rs` to read memory non-intrusively via `cortex.mem_read_block` without forcing `cortex.halt()`.
- [x] 2.2 Verify `GET /api/v1/programmer/read` returns valid flash hex payload at `0x08000000` via live REST API.
- [x] 2.3 Verify `POST /api/v1/programmer/erase` and `POST /api/v1/programmer/flash` correctly halt, un-gate DBGMCU, program, verify, and resume on STM32H5.

## 3. GDB Server Dual-AP Routing

- [x] 3.1 Update `firmware-rs/src/programmer/gdb_server.rs` to inspect target DPIDR on connection and configure `cortex.select_ap(1)` with `csw = 0x23000012` for ARMv8-M / Cortex-M33 targets.
- [x] 3.2 Add unit test in `crates/agentla-programmer/tests/programmer_tests.rs` verifying CortexM dual-AP selection and memory read/write behavior.
- [x] 3.3 Verify live GDB client connection over TCP port 3333 inspecting registers and reading memory at `0x08000000`.

## 4. Hardware Verification & Change Archival

- [x] 4.1 Run full CMSIS-DAP verification suite (`test/test_cmsis_dap.py`) to confirm zero regressions.
- [x] 4.2 Run cargo test on `agentla-programmer` and `agentla-targets`.
- [x] 4.3 Build release firmware, package OTA binary, deploy to Slot B, and verify live.
- [x] 4.4 Sync OpenSpec specs, archive change `stm32h5-programmer-and-gdb-dual-ap`, and commit changes.
