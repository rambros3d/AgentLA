## 1. Implement Debug Server Crate (`agentla-debug-server`)

- [x] 1.1 Implement GDB Remote Serial Protocol parser, checksum calculation, and packet formatter
- [x] 1.2 Implement GDB command dispatcher and register/memory serializers
- [x] 1.3 Implement SEGGER RTT control block detection and circular buffer extraction
- [x] 1.4 Add host unit tests (`tests/gdb_rsp.rs`, `tests/rtt_tests.rs`) on x86_64

## 2. Firmware Integration (`firmware-rs`)

- [x] 2.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-debug-server`
- [x] 2.2 Refactor `firmware-rs/src/programmer/gdb_server.rs` to delegate RSP protocol parsing to `agentla-debug-server`
- [x] 2.3 Refactor `firmware-rs/src/programmer/rtt.rs` to delegate RTT buffer parsing to `agentla-debug-server`

## 3. Verification

- [x] 3.1 Run `cargo test --package agentla-debug-server` on host x86_64
- [x] 3.2 Build bare-metal firmware `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 3.3 Validate zero regressions
