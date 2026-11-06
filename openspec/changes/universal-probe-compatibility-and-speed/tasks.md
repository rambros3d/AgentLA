## 1. Phase 1: USB CMSIS-DAP v2 Bulk Interface

- [x] 1.1 Add Microsoft OS 2.0 WinUSB descriptors and CMSIS-DAP v2 interface to `firmware-rs/src/usb.rs`
- [x] 1.2 Implement bulk endpoint packet handler and `DAP_TransferBlock` / `DAP_SWJ_Sequence` in `firmware-rs/src/programmer/dap.rs`
- [x] 1.3 Add unit tests in `test/test_cmsis_dap_v2.py` verifying protocol responses and block transfers

## 2. Phase 2: Segger RTT Background Logging Engine

- [x] 2.1 Implement RTT signature scanner (`"SEGGER RTT"`) and `SEGGER_RTT_CB` parser in `firmware-rs/src/programmer/rtt.rs`
- [x] 2.2 Implement non-intrusive SWD buffer drain loop and link to WebSocket / SSE streaming
- [x] 2.3 Expose `/api/v1/programmer/rtt/status` and `/api/v1/programmer/rtt/stream` in `firmware-rs/src/rest.rs`
- [x] 2.4 Add unit tests in `test/test_rtt_telemetry.py`

## 3. Phase 3: On-Probe Network GDB Server (TCP :3333)

- [x] 3.1 Implement GDB Remote Serial Protocol framing, checksums, and parser in `firmware-rs/src/programmer/gdb_server.rs`
- [x] 3.2 Wire GDB RSP commands (`$g`, `$G`, `$m`, `$M`, `$s`, `$c`, `$Z0`, `$z0`) to `CortexM` and FPB registers
- [x] 3.3 Spawn TCP server listening on `10.10.10.10:3333` in `firmware-rs/src/main.rs`
- [x] 3.4 Add unit tests in `test/test_gdb_server.py`

## 4. Phase 4: Concurrent Double-Buffered Turbo Flashing

- [x] 4.1 Update `FlashAlgorithmRunner` in `firmware-rs/src/programmer/algo/mod.rs` to support dual-buffer ping-pong execution
- [x] 4.2 Overlap SWD block writes for page $N+1$ while target core executes `ProgramPage` for page $N$
- [x] 4.3 Add unit test in `test/test_turbo_flash.py`

## 5. Phase 5: Nordic CoreSight CTRL-AP Unprotect

- [x] 5.1 Add CoreSight AP selection (`APSEL`) in `firmware-rs/src/programmer/target/cortex_m.rs`
- [x] 5.2 Implement Nordic CTRL-AP `ERASEALL` mass-erase routine in `firmware-rs/src/programmer/target/nrf.rs`
- [x] 5.3 Add REST API `POST /api/v1/programmer/rescue/nrf` in `firmware-rs/src/rest.rs`
- [x] 5.4 Add unit test in `test/test_nrf_rescue.py`
