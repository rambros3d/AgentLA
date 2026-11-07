## Why

The GPIO pin ownership registry in `firmware-rs` (`pin_owner.rs`) is a 626-line subsystem responsible for tracking in-RAM peripheral claims, enforcing mutual exclusion (e.g. preventing I2C from claiming pins already claimed by PWM or Programmer), handling all-or-nothing multi-pin claims, and orchestrating safe-state transitions.

Extracting this into `crates/agentla-pin-arbiter` decouples the logical claim engine and capability verification from chip-specific register writes (`pac::IO_BANK0`). It enables:
- Pure desktop host unit testing on `x86_64` for claim/release race conditions, conflict detection, and handle lifecycles.
- Reusing the peripheral pin arbiter pattern across other embedded projects and board variants.

## What Changes

- **Add `crates/agentla-pin-arbiter`**:
  - Implements `PinArbiter` registry with fixed slot pools, capability masks (`CAP_SIO`, `CAP_PWM`, `CAP_SPI`, `CAP_UART`, `CAP_I2C`, `CAP_ADC`, `CAP_PIO`).
  - Implements all-or-nothing atomic claim transactions and conflict detection.
  - Implements RAII `SlotId` handles and driver ownership queries (`owner_of`, `handle_pins`, `has_claim`).
  - Defines `PinQuiesce` trait for platform-specific pin reset on release.
  - Native `#![no_std]` with optional `defmt` and `std` features.
  - Comprehensive host unit tests on `x86_64`.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-pin-arbiter`.
  - Refactors `firmware-rs/src/pin_owner.rs` to delegate logical arbitration to `agentla-pin-arbiter` while executing RP2350 SIO/pad register quiescing.

## Capabilities

### New Capabilities
- `domain-crate-pin-arbiter`: Pure `#![no_std]` GPIO pin ownership and conflict arbitration engine.

### Modified Capabilities
<!-- None: REST API and runtime pin safety behaviors remain 100% identical. -->

## Impact

- **Codebase**: `firmware-rs/src/pin_owner.rs` delegates state and conflict checks to `agentla-pin-arbiter`.
- **Cargo Workspace**: `crates/agentla-pin-arbiter` implemented with unit tests.
