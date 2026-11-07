## 1. Workspace Foundation

- [x] 1.1 Add `crates/agentla-analyzer` and `crates/agentla-programmer` to root `Cargo.toml` workspace members
- [x] 1.2 Scaffold crate directory structures under `crates/agentla-analyzer` and `crates/agentla-programmer`

## 2. Logic Analyzer & Decoders Crate (`agentla-analyzer`)

- [x] 2.1 Create `crates/agentla-analyzer/Cargo.toml` with `no_std` configuration and optional `std` feature for tests
- [x] 2.2 Migrate capture state machines and trigger types (`State`, `Mode`, `TriggerType`, `TriggerConfig`)
- [x] 2.3 Migrate bit-delta RLE block compression, decompression, and transform algorithms
- [x] 2.4 Migrate VCD (Value Change Dump) formatting and text generation
- [x] 2.5 Migrate protocol decoders (I2C, SPI, CAN, SWD, 1-wire, I3C) into `agentla-analyzer::decoders`
- [x] 2.6 Add host unit tests (`tests/compression.rs`, `tests/decoders.rs`) verifying algorithms on x86_64

## 3. Programmer Protocols Crate (`agentla-programmer`)

- [x] 3.1 Create `crates/agentla-programmer/Cargo.toml` with `no_std`, mock traits, and optional `rp2350-pio` feature
- [x] 3.2 Define `SwdIo` and `SwioIo` hardware-independent transport traits
- [x] 3.3 Migrate CMSIS-DAP v1/v2 packet handling and dispatch logic (`dap.rs`)
- [x] 3.4 Migrate Arm ADIv5 SWD protocol engine and sequence handlers (`swd.rs`)
- [x] 3.5 Migrate WCH QingKe 1-wire SWIO and 2-wire RVSWD protocol engines (`swio.rs`, `rvswd.rs`)
- [x] 3.6 Migrate target identification catalog and CMSIS FLM flash algorithm runner (`target/`, `algo/`)
- [x] 3.7 Add PIO state machines and FIFO drivers under `agentla-programmer::pio` gated by `rp2350-pio`
- [x] 3.8 Add host unit tests verifying CMSIS-DAP packet handling and ADIv5 state machines using mock transports

## 4. Firmware Integration (`firmware-rs`)

- [x] 4.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-analyzer` and `agentla-programmer` (with `features = ["rp2350-pio"]`)
- [x] 4.2 Refactor `firmware-rs/src/analyzer.rs` to delegate compression, state, and decoders to `agentla-analyzer`
- [x] 4.3 Refactor `firmware-rs/src/programmer/*` to delegate protocol logic and DAP handling to `agentla-programmer`
- [x] 4.4 Remove redundant duplicate implementations from `firmware-rs`

## 5. Build & Test Verification

- [x] 5.1 Run `cargo test --package agentla-analyzer --package agentla-programmer` on host x86_64
- [x] 5.2 Build bare-metal firmware with `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 5.3 Validate binary size and memory layout against memory.x limits
