## Why

Signal analysis, oscilloscope metrics computation, waveform frequency divider math, and multi-channel servo/PWM integer arithmetic currently reside directly inside `firmware-rs` (`src/instruments/oscilloscope.rs`, `src/instruments/generator.rs`, and `src/servo.rs`). These mathematical and digital signal algorithms are pure functions operating on memory slices and integer parameters. Extracting them into a standalone `#![no_std]` crate (`crates/agentla-instruments`) enables:
- Pure desktop host unit testing on x86_64 for waveform math and frequency calculations.
- Clean separation between Embassy hardware peripheral setup (ADC DMA, hardware PWM slices) and digital signal analysis.
- Reusability of instrument algorithms across different microcontroller architectures.

## What Changes

- **Add `crates/agentla-instruments`**:
  - Implements oscilloscope waveform analysis: peak-to-peak voltage ($V_{pp}$), RMS voltage ($V_{rms}$), zero-crossing frequency detection, and duty cycle calculation on ADC sample slices `&[u16]`.
  - Implements digital edge and threshold triggering state detection on sample streams.
  - Implements function generator frequency divider math: converts target frequency ($f_{target}$) and system clock ($f_{sys}$) into 8.4 fractional divider and 16-bit TOP wrap values.
  - Implements unified servo and generic PWM timing arithmetic: 50 Hz fast-path calculation, pulse-width microsecond clamping, and fixed-point divisor search.
  - Native `#![no_std]` with optional `defmt` and `std` features.
  - Comprehensive host unit tests verifying arithmetic precision and edge trigger logic.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-instruments`.
  - Refactors `oscilloscope.rs`, `generator.rs`, and `servo.rs` to delegate mathematical calculations to `agentla-instruments`.

## Capabilities

### New Capabilities
- `domain-crate-instruments`: Pure `#![no_std]` instrument signal processing, waveform metrics, and PWM timing math.

### Modified Capabilities
<!-- None: External REST and WebUI instrument behavior remains 100% identical. -->

## Impact

- **Codebase**: `firmware-rs/src/instruments/{oscilloscope, generator}.rs` and `firmware-rs/src/servo.rs` delegate math to `agentla-instruments`.
- **Cargo Workspace**: `crates/agentla-instruments` fully implemented with host unit tests.
