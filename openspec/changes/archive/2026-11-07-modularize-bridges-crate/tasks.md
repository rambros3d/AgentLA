## 1. Workspace Setup

- [x] 1.1 Add `crates/agentla-bridges` to workspace members in root `Cargo.toml`
- [x] 1.2 Scaffold crate directory structure under `crates/agentla-bridges`

## 2. Implement Bridges Crate (`agentla-bridges`)

- [x] 2.1 Create `crates/agentla-bridges/Cargo.toml` with `no_std`, `embedded-hal`, `embedded-can`, and optional `defmt`/`std`
- [x] 2.2 Implement `i2c_tinyusb` protocol types, constants, and `I2cTinyUsbHandler`
- [x] 2.3 Implement `gs_usb` protocol types, `gs_host_frame`, bittiming calculations, and mode handling
- [x] 2.4 Implement `slcan` ASCII parser, command types, and frame serializer
- [x] 2.5 Add host unit tests (`tests/i2c_tinyusb.rs`, `tests/gs_usb.rs`, `tests/slcan.rs`) on x86_64

## 3. Firmware Integration (`firmware-rs`)

- [x] 3.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-bridges`
- [x] 3.2 Refactor `firmware-rs/src/bridge/i2c_tinyusb.rs` to delegate protocol handling to `agentla-bridges`
- [x] 3.3 Refactor `firmware-rs/src/bridge/can_gsusb.rs` to delegate frame and request handling to `agentla-bridges`
- [x] 3.4 Refactor `firmware-rs/src/bridge/slcan.rs` to delegate ASCII parsing to `agentla-bridges`

## 4. Verification

- [x] 4.1 Run `cargo test --package agentla-bridges` on host x86_64
- [x] 4.2 Build bare-metal firmware `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 4.3 Validate memory size and zero regressions
