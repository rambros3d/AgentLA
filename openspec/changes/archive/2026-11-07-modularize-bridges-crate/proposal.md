## Why

The USB bridge subsystem in `firmware-rs` currently implements three major host adapter protocols:
1. Linux SocketCAN `gs_usb` protocol (`can_gsusb.rs`)
2. Linux kernel `i2c-tiny-usb` protocol (`i2c_tinyusb.rs`)
3. Lawicel ASCII CAN bridge protocol (`slcan.rs`)

Currently, these protocol parsers and packet serializers are coupled directly to `firmware-rs` and board-specific pins (`ActiveBoard::ADAPTERS.can`, `pac::PADS_BANK0`). Extracting them into an Embassy-aligned `#![no_std]` crate (`crates/agentla-bridges`) operating over standard `embedded-hal` / `embedded-hal-async`, `embedded-io-async`, and `embedded-can` traits enables:
- Reusability across any Embassy microcontroller (RP2040, RP2350, STM32, ESP32).
- Pure host-based unit testing on `x86_64` without hardware.
- Decoupling Linux USB protocol logic from MCU GPIO/pin ownership logic.

## What Changes

- **Add `crates/agentla-bridges`**:
  - Implements Linux `i2c-tiny-usb` protocol dispatcher over `embedded-hal` / `embedded-hal-async` I2C traits.
  - Implements Linux SocketCAN `gs_usb` command handler, bit timing calculation, and frame packing conforming to `gs_usb.h`.
  - Implements Lawicel SLCAN ASCII command parser and frame serializer over `embedded-io-async` / byte slices.
  - Native `#![no_std]` with optional `defmt` and `std` features.
  - Host unit tests verifying protocol compliance against Linux kernel driver specifications.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-bridges`.
  - Replaces internal bridge protocol implementations with delegation to `agentla-bridges`.
  - Retains physical hardware pin setup, pin ownership arbitration, and USB endpoint wiring in `firmware-rs`.
- **Update Workspace Root**:
  - Adds `"crates/agentla-bridges"` to workspace members in root `Cargo.toml`.

## Capabilities

### New Capabilities
- `domain-crate-bridges`: Embassy-aligned USB bridge protocol engines for Linux `gs_usb`, `i2c-tiny-usb`, and SLCAN operating over standard embedded traits.

### Modified Capabilities
<!-- None: External USB protocol behavior remains 100% compatible with Linux kernel drivers and host tooling. -->

## Impact

- **Codebase**: `firmware-rs/src/bridge/{can_gsusb, i2c_tinyusb, slcan}.rs` refactored to delegate to `agentla-bridges`.
- **Cargo Workspace**: New member `crates/agentla-bridges`.
- **CI / Testing**: New host tests runnable via `cargo test --package agentla-bridges`.
