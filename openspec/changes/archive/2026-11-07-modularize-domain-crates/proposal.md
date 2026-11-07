## Why

Currently, the `firmware-rs` crate is a 15,000+ line bare-metal monolith targeting `thumbv8m.main-none-eabihf`. Core domain capabilities—including protocol decoders, sample compression, VCD generation, CMSIS-DAP packet processing, Arm ADIv5 SWD, WCH SWIO/RVSWD, and flash algorithm loaders—are coupled directly to MCU hardware (`rp-pac`, `embassy-rp`). This makes host unit testing on desktop (`cargo test --target x86_64`) impossible, slows down build times, and prevents reusing protocols across tools, boards, or desktop host utilities.

Extracting these domain capabilities into two dedicated `#![no_std]` workspace crates (`agentla-analyzer` and `agentla-programmer`) under `crates/` establishes clean architectural boundaries, unlocks host-based CI test coverage, and dramatically improves code maintainability.

## What Changes

- **Add `crates/agentla-analyzer`**:
  - Encapsulates capture configuration, capture state machine, software trigger detection (edge, pattern, level).
  - Encapsulates bit-delta run-length encoding (RLE) block compression and decompression.
  - Encapsulates Value Change Dump (VCD) text stream formatting.
  - Encapsulates pure-logic bus protocol decoders (I2C, SPI, CAN, SWD, 1-wire, I3C).
  - Native `#![no_std]` with optional `std` feature for comprehensive host unit tests on x86_64/ARM64.
- **Add `crates/agentla-programmer`**:
  - Defines hardware-agnostic `SwdIo` and `SwioIo` transport traits.
  - Encapsulates CMSIS-DAP v1/v2 packet processing and command dispatching.
  - Encapsulates Arm ADIv5 SWD protocol state machine (DP/AP registers, turnaround, sequence control).
  - Encapsulates WCH QingKe 1-wire (SWIO) and 2-wire (RVSWD) debug module protocols.
  - Encapsulates target identification catalog and CMSIS FLM flash algorithm execution engine.
  - Includes optional `rp2350-pio` feature providing high-speed PIO state machine implementations directly in the crate.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-analyzer` and `agentla-programmer`.
  - Replaces internal duplicate protocol/analyzer logic with calls to the crate implementations.
  - Adapts hardware peripheral setup to wire board GPIO/PIO into the crate traits and drivers.
- **Update Workspace Root**:
  - Adds `"crates/agentla-analyzer"` and `"crates/agentla-programmer"` to the workspace members in root `Cargo.toml`.

## Capabilities

### New Capabilities
- `domain-crate-analyzer`: Encapsulates logic analyzer capture state, bit-delta compression, VCD generation, and bus decoders into an independent `#![no_std]` crate runnable in host unit tests.
- `domain-crate-programmer`: Encapsulates SWD, SWIO, RVSWD, CMSIS-DAP packet handling, target detection, and flash algorithms behind abstract transport traits with optional RP2350 PIO support.

### Modified Capabilities
<!-- None: External functional behaviors of the logic analyzer, CMSIS-DAP, and flash programmer remain identical to existing specs; this is an architectural modularization of the firmware engine. -->

## Impact

- **Codebase**: `firmware-rs/src/analyzer.rs`, `firmware-rs/src/decoder.rs`, and `firmware-rs/src/programmer/*` will be migrated into `crates/agentla-analyzer` and `crates/agentla-programmer`.
- **Cargo Workspace**: `Cargo.toml` workspace members list expanded.
- **CI / Testing**: New host-runnable test suites added under `crates/agentla-analyzer/tests` and `crates/agentla-programmer/tests`, allowing rapid regression testing in GitHub Actions without hardware or QEMU.
- **Firmware Binary**: Zero regressions to runtime functionality; binary size and performance remain optimized.
