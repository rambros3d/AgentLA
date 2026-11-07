## 1. Implement Instruments Crate (`agentla-instruments`)

- [x] 1.1 Implement oscilloscope waveform analysis functions (Vpp, Vrms, frequency, duty cycle, threshold triggers)
- [x] 1.2 Implement signal generator frequency divider and TOP count math
- [x] 1.3 Implement servo PWM integer arithmetic (50 Hz fast path, arbitrary frequency search)
- [x] 1.4 Add host unit tests (`tests/scope_metrics.rs`, `tests/pwm_math.rs`, `tests/servo_math.rs`) on x86_64

## 2. Firmware Integration (`firmware-rs`)

- [x] 2.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-instruments`
- [x] 2.2 Refactor `firmware-rs/src/instruments/oscilloscope.rs` to delegate metrics calculation to `agentla-instruments`
- [x] 2.3 Refactor `firmware-rs/src/instruments/generator.rs` to delegate divider math to `agentla-instruments`
- [x] 2.4 Refactor `firmware-rs/src/servo.rs` to delegate PWM timing math to `agentla-instruments`

## 3. Verification

- [x] 3.1 Run `cargo test --package agentla-instruments` on host x86_64
- [x] 3.2 Build bare-metal firmware `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 3.3 Validate zero regressions
