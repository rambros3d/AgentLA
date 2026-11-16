## 1. Target Voltage Provider Hook in agentla-programmer

- [x] 1.1 Add `voltage_provider: Option<fn() -> u32>` and `set_voltage_provider()` to `StLinkHandler` in `crates/agentla-programmer/src/stlink.rs`.
- [x] 1.2 Update `STLINK_GET_TARGET_VOLTAGE` (`0xF7`) in `StLinkHandler::execute` to evaluate the provider, clamp voltages under 800 mV to 0 mV, and return 8-byte payload (`2400` reference factor and `v_target_mv`).
- [x] 1.3 Add unit tests in `crates/agentla-programmer` testing `0xF7` with 3.3V, 1.8V, unpowered floating (<800 mV clamped to 0), and fallback defaults.

## 2. Hardware ADC Integration in firmware-rs

- [x] 2.1 Expose `get_target_voltage_mv() -> u32` in `firmware-rs/src/telemetry.rs` sampling ADC channel 2.
- [x] 2.2 Hook `get_target_voltage_mv` into the active `StLinkHandler` in `firmware-rs/src/usb.rs`.

## 3. Verification & Testing

- [x] 3.1 Run unit tests with `cargo test --target x86_64-unknown-linux-gnu -p agentla-programmer`.
- [x] 3.2 Verify embedded build with `cargo check --target thumbv8m.main-none-eabihf`.
