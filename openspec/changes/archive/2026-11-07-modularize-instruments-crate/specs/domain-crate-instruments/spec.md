## ADDED Requirements

### Requirement: Standalone Instruments Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-instruments` configured as a `#![no_std]` library with optional `defmt` and `std` features.

#### Scenario: Compiling and testing on host targets
- **WHEN** building `agentla-instruments` on an x86_64 host with `cargo test --package agentla-instruments`
- **THEN** all waveform calculation and PWM timing unit tests compile and pass without MCU HAL dependencies.

### Requirement: Oscilloscope Waveform Metrics Computation
The `agentla-instruments` crate SHALL compute $V_{pp}$, $V_{rms}$, frequency, and duty cycle from raw 12-bit ADC sample buffers without dynamic heap allocation.

#### Scenario: Computing metrics for synthetic sine wave
- **WHEN** a buffer of synthetic sine wave samples is evaluated by `compute_metrics`
- **THEN** the calculated $V_{pp}$, $V_{rms}$, and zero-crossing frequency match the expected mathematical values within 2% margin.

### Requirement: PWM Frequency and Divider Calculation
The `agentla-instruments` crate SHALL calculate integer and fractional clock divisors and TOP wrap counts for arbitrary target frequencies given a system clock.

#### Scenario: Calculating 50 Hz servo PWM parameters
- **WHEN** calculating timing for 50 Hz with 150 MHz system clock
- **THEN** the calculator yields integer divisor 150, fractional divisor 0, and top count 19,999 (1 µs resolution).

#### Scenario: Calculating high-frequency square wave parameters
- **WHEN** calculating timing for 10 MHz with 150 MHz system clock
- **THEN** the calculator yields integer divisor 1, fractional divisor 0, and top count 14 (50% duty at count 7).
