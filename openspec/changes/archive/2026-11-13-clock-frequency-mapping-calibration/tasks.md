## 1. Clock Calculation & Mapping Core

- [x] 1.1 Implement `crates/agentla-programmer/src/clock.rs` containing `calculate_pio_clkdiv`, `swd_divisor_to_khz`, `swd_khz_to_divisor`, and `wch_speed_to_khz`.
- [x] 1.2 Add unit tests in `crates/agentla-programmer` validating divider math across clock ranges (5 kHz to 10 MHz) and protocol mapping tables.

## 2. PIO Transport Fractional Dividers

- [x] 2.1 Update `SwdTransport::set_clock` in `crates/agentla-programmer/src/pio/swd.rs` to write both integer and fractional dividers into `pac::PIO0.sm(0).clkdiv()`.
- [x] 2.2 Implement `set_clock(&mut self, freq_khz: u32)` on `RvswdTransport` in `crates/agentla-programmer/src/pio/rvswd.rs` writing integer and fractional dividers to `pac::PIO0.sm(1).clkdiv()`.

## 3. Protocol Handlers Integration

- [x] 3.1 Update `DBG_APIV2_SWD_SET_FREQ` in `crates/agentla-programmer/src/stlink.rs` to map divisor values with `swd_divisor_to_khz`.
- [x] 3.2 Update command `0x0C` in `crates/agentla-programmer/src/wch.rs` to translate speed tiers using `wch_speed_to_khz` and configure transport clock.

## 4. Verification & Testing

- [x] 4.1 Run unit tests with `cargo test --target x86_64-unknown-linux-gnu -p agentla-programmer`.
- [x] 4.2 Verify embedded build with `cargo check --target thumbv8m.main-none-eabihf`.
