## Context

In ARM and RISC-V embedded debugging, the debug probe needs to know if the target board has power applied. STM32CubeIDE, STM32CubeProgrammer, and OpenOCD query target voltage before initiating debug connections. In the ST-Link protocol, command byte `0xF7` (`STLINK_GET_TARGET_VOLTAGE`) expects an 8-byte response:
- Bytes 0..3: ADC calibration / reference constant (`2400` / `0x0960`).
- Bytes 4..7: Target voltage in millivolts (e.g. `3300` for 3.3V, `1800` for 1.8V, `0` for unpowered).

AgentLA's hardware includes an ADC channel connected to the Vtarget sense rail (RP2350 ADC channel 2). Currently, `telemetry.rs` reads this channel for telemetry, but `StLinkHandler` returns a hardcoded `3300 mV`, which obscures whether the target is actually powered.

## Goals / Non-Goals

**Goals:**
- Provide a zero-allocation, `#![no_std]` target voltage provider hook in `StLinkHandler`.
- Wire `telemetry::sample_telemetry().v_target_mv` (or direct ADC reading) to `StLinkHandler::set_target_voltage_fn`.
- When command `0xF7` is received, dynamically return:
  - Bytes 0..3: `2400` (little-endian u32)
  - Bytes 4..7: `v_target_mv` (little-endian u32)
- Apply a noise floor cutoff: if `v_target_mv < 800`, return `0 mV` so host IDEs reliably identify an unpowered target.
- Verify live target voltage queries in unit tests and integration tests.

**Non-Goals:**
- Providing software voltage regulation or driving target power from the sense line (sense line is input only).

## Decisions

### Decision 1: Function Pointer Provider Hook

- **Choice**: Add `voltage_provider: Option<fn() -> u32>` to `StLinkHandler`.
- **Rationale**:
  - Completely `#![no_std]` and zero-heap.
  - Allows `firmware-rs` to supply the live hardware ADC reading function, while host unit tests can supply mock voltage functions.
  - Safe to call concurrently from the USB task without lock contention.

### Decision 2: 800 mV Floating Noise Threshold

- **Choice**: Clamp readings below 800 mV to 0 mV.
- **Rationale**:
  - High-impedance ADC input pins can pick up 100-300 mV of capacitive floating voltage when no target is connected. Clamping below 800 mV ensures host IDEs show "Target 0.00V" rather than false millivolts.

## Risks / Trade-offs

- **[Risk] ADC conversion delay inside USB task**
  - *Mitigation*: The USB task reads a cached atomic snapshot updated by the telemetry sampler or performs a single synchronous ADC conversion (< 2 µs on RP2350), which does not delay USB transaction deadlines.
