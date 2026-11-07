## 1. Implement Pin Arbiter Crate (`agentla-pin-arbiter`)

- [x] 1.1 Implement driver roles, capabilities, and error types
- [x] 1.2 Implement `PinArbiter` registry with all-or-nothing transactions
- [x] 1.3 Implement `PinQuiesce` trait and release handlers
- [x] 1.4 Add host unit tests (`tests/claim_tests.rs`) on x86_64

## 2. Firmware Integration (`firmware-rs`)

- [x] 2.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-pin-arbiter`
- [x] 2.2 Refactor `firmware-rs/src/pin_owner.rs` to delegate arbitration to `agentla-pin-arbiter`

## 3. Verification

- [x] 3.1 Run `cargo test --package agentla-pin-arbiter` on host x86_64
- [x] 3.2 Build bare-metal firmware `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 3.3 Validate zero regressions
