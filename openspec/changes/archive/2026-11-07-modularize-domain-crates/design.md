## Context

The AgentLA firmware workspace currently has one BSP crate (`crates/agentla-bsp`), several board crates (`boards/pico2`, `boards/bakuprobe`, `boards/templates`), and the primary monolithic `firmware-rs` firmware binary. Inside `firmware-rs`, over 7,500 lines of logic implement the logic analyzer core, signal decoders, CMSIS-DAP protocol handling, SWD ADIv5 state machines, WCH QingKe 1-wire/2-wire debug protocols, target diagnostics, and flash algorithm loaders.

Because `firmware-rs` targets `thumbv8m.main-none-eabihf` and links `cortex-m`, `cortex-m-rt`, and `rp-pac`, none of these high-value algorithms and protocol parsers can be compiled or unit-tested on host systems (`x86_64` Linux, macOS, or GitHub Actions CI).

This design introduces two cohesive domain crates into `crates/`:
1. `crates/agentla-analyzer`: Pure logic, sample compression, VCD generation, and protocol decoders.
2. `crates/agentla-programmer`: Hardware-independent debug protocols, CMSIS-DAP packet dispatcher, target identification, and flash algorithms, with optional RP2350 PIO drivers feature-gated.

```
┌────────────────────────────────────────────────────────────────────────┐
│                              firmware-rs                               │
│        (RP2350 runtime, USB stacks, Embassy networking, WebUI)         │
└───────────────────────┬───────────────────────────────┬────────────────┘
                        │ depends on                    │ depends on (feature="rp2350-pio")
                        ▼                               ▼
┌───────────────────────────────────────┐ ┌───────────────────────────────────────┐
│        crates/agentla-analyzer        │ │       crates/agentla-programmer       │
│                                       │ │                                       │
│ • config.rs: CaptureConfig, State     │ │ • traits.rs: SwdIo, SwioIo traits     │
│ • compression.rs: bit_delta RLE       │ │ • dap/: CMSIS-DAP v1/v2 packet parser │
│ • vcd.rs: VCD stream formatter        │ │ • swd/: Arm ADIv5 DP/AP state machine │
│ • trigger.rs: Edge / pattern search   │ │ • swio/: WCH QingKe 1-wire protocol   │
│ • decoders/: I2C, SPI, CAN, SWD, etc. │ │ • rvswd/: WCH 2-wire protocol         │
│                                       │ │ • targets/: Target ID & diagnostics   │
│ (Pure #![no_std], std for tests)      │ │ • algo/: FLM flash loader & runner    │
│                                       │ │ • pio/: Feature="rp2350-pio" drivers  │
└───────────────────────────────────────┘ └───────────────────────────────────────┘
                        ▲                               ▲
                        │                               │
            ┌───────────┴───────────────────────────────┴───────────┐
            │             Host Unit Tests (cargo test)              │
            │     Runs natively on Linux x86_64 / CI in seconds     │
            └───────────────────────────────────────────────────────┘
```

## Goals / Non-Goals

**Goals:**
- Extract logic analyzer capture state, bit-delta RLE compression/decompression, VCD export, and protocol decoders into `crates/agentla-analyzer`.
- Extract SWD, SWIO, RVSWD, CMSIS-DAP packet handling, target detection, and flash algorithm runner into `crates/agentla-programmer`.
- Ensure both crates are `#![no_std]` by default with an optional `std` feature enabling comprehensive host `cargo test` suites on `x86_64-unknown-linux-gnu`.
- Provide an optional `rp2350-pio` feature in `agentla-programmer` that contains the tested PIO programs and low-level hardware drivers, so `firmware-rs` remains clean and future RP2350 boards can use them directly.
- Refactor `firmware-rs` to delegate to these crates with zero functional regression or memory overhead.

**Non-Goals:**
- Creating fine-grained micro-crates for every individual decoder or chip target (e.g. no `agentla-decoder-can` or `agentla-target-stm32`). Domain bundling keeps Cargo configuration maintainable.
- Changing the public REST API, WebUI protocol, or USB endpoint behavior of the firmware.
- Rewriting the PIO assembly instructions themselves—only relocating and wrapping them cleanly.

## Decisions

### Decision 1: Domain-Oriented Bundling (`agentla-analyzer` and `agentla-programmer`)
- **Rationale**: Keeps the workspace clean and cohesive. `agentla-analyzer` handles all passive signal ingestion, compression, and interpretation. `agentla-programmer` handles all active target debugging, flashing, and probe interactions.
- **Alternatives Considered**:
  - *Micro-crates (4-6 crates)*: High maintenance overhead, excessive `Cargo.toml` files, version sync complexity.
  - *Keep in `firmware-rs` with cfg flags*: Brittle, fails to provide clean dependency boundaries, still blocks pure desktop testing of decoders.

### Decision 2: Feature-gated PIO drivers (`rp2350-pio`)
- **Rationale**: Putting PIO programs directly in `agentla-programmer` under `[features] rp2350-pio = ["dep:rp-pac"]` makes the crate batteries-included for RP2350 while keeping default builds free of hardware dependencies.
- **Alternatives Considered**:
  - *Strict trait-only (PIO strictly in `firmware-rs`)*: Forces any other board or application to duplicate the 200+ lines of PIO assembly and FIFO pump code.
  - *Separate `agentla-programmer-rp2350` crate*: Adds another crate layer without significant benefit over a feature flag.

### Decision 3: Abstract Transport Traits for Programmer
- **Rationale**: Defines `SwdIo` and `SwioIo` traits inside `agentla-programmer`:
  ```rust
  pub trait SwdIo {
      type Error;
      fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error>;
      fn reset_sequence(&mut self) -> Result<(), Self::Error>;
  }
  ```
  CMSIS-DAP packet handlers and Cortex-M debug sessions operate over `T: SwdIo`. In unit tests on x86_64, mock transports simulate DP/AP reads and writes without physical hardware.

## Risks / Trade-offs

- **[Risk] Compile-time breakage if `rp-pac` leaks into default features**
  → *Mitigation*: The default features in `crates/agentla-programmer/Cargo.toml` will explicitly exclude `rp2350-pio`. Root CI will run `cargo test --package agentla-programmer --package agentla-analyzer` on the host to guarantee purity.
- **[Risk] Code drift between `firmware-rs` and the new crates during migration**
  → *Mitigation*: Move modules in complete units and verify both bare-metal build (`cargo build --target thumbv8m.main-none-eabihf`) and host tests after each crate integration.
- **[Risk] Code size / LTO impacts**
  → *Mitigation*: Workspace-level LTO and optimization flags (`opt-level = "s"`) in root `Cargo.toml` already inline cross-crate boundaries. Verified that Rust trait dispatch can be static (`impl SwdIo` / monomorphization) without dynamic `dyn` overhead.

## Migration Plan

1. Create `crates/agentla-analyzer` with unit tests for compression, VCD generation, and decoders.
2. Add `crates/agentla-analyzer` to root `Cargo.toml` workspace and integrate into `firmware-rs`.
3. Create `crates/agentla-programmer` with transport traits, CMSIS-DAP handler, SWD, SWIO, targets, flash algos, and feature-gated PIO.
4. Add `crates/agentla-programmer` to root `Cargo.toml` workspace and integrate into `firmware-rs`.
5. Remove obsolete internal duplicate code from `firmware-rs/src/programmer` and `firmware-rs/src/decoder.rs`.
6. Verify host tests pass on `x86_64` and release build succeeds for `thumbv8m.main-none-eabihf`.
