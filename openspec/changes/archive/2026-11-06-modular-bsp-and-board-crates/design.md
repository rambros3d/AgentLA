# Design: Modular Board Support Package (BSP) Architecture

## Context

AgentLA currently targets the Raspberry Pi Pico 2 (RP2350A, 30 GPIOs), but hardware variants such as BakuProbe / AgentProbe (RP2350B, 48 GPIOs, level shifters, current sensing, programmable gain front-ends) require different pinouts, transceivers, and analog signal paths. Without a clean Board Support Package (BSP) abstraction, supporting multiple boards leads to `#cfg[feature]` spaghetti, pin collision risks, and tight coupling between driver logic and physical PCB layout.

---

## Goals & Non-Goals

### Goals
- Establish an idiomatic embedded Rust **Board Support Package (BSP)** workspace architecture.
- Define a shared trait contract (`BoardProfile`) that models physical pin zones:
  - Fixed Programmer Zone (SWD, RVSWD, SWIO, UART)
  - Dedicated Transceivers (CAN-FD, RS-485)
  - Flexible Instrument Pool (Logic Analyzer, Signal Generator, WS2812, Glitcher)
  - Analog Front-End (Direct ADC, PGA113, INA180 current sense)
- Create a dedicated library crate `crates/agentla-bsp` hosting reusable external IC drivers (TI PGA113, TI INA180, TCAN334, TXS0108E).
- Isolate board configurations into lightweight board crates (`boards/pico2`, `boards/bakuprobe`, `boards/templates`).
- Automatically generate `/api/board/manifest` and pin metadata from the active board crate.

### Non-Goals
- Runtime hot-swapping of board architectures on a running microcontroller (the board identity is a compile-time target).
- Support for non-RP2350 microcontrollers (RP2350A and RP2350B remain the core silicon targets).

---

## Architecture & Module Hierarchy

```
AgentLA-rust/
├── Cargo.toml (root workspace)
├── crates/
│   └── agentla-bsp/                  # Core Board Support Package Library
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── traits.rs             # BoardProfile, ProgrammerPins, InstrumentPool
│           └── drivers/              # Hardware-agnostic IC drivers
│               ├── pga113.rs         # TI PGA113 Programmable Gain Amplifier (SPI)
│               ├── ina180.rs         # TI INA180 Current Sense Amplifier
│               ├── tcan334.rs        # TCAN334 CAN-FD transceiver control
│               └── txs0108.rs        # TXS0108E Level shifter control
│
├── boards/
│   ├── pico2/                        # Raspberry Pi Pico 2 Board Crate
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                # Implements BoardProfile for Pico 2
│   │   └── board.json                # Web desktop asset & manifest
│   │
│   ├── bakuprobe/                    # BakuProbe Instrument Board Crate
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs                # Implements BoardProfile for RP2350B + PGA113
│   │   └── board.json
│   │
│   └── templates/                    # Boilerplate template for new PCBs
│       ├── Cargo.toml
│       └── src/lib.rs
│
└── firmware-rs/                      # Application Firmware Binary
    ├── Cargo.toml                    # Depends on active board crate
    └── src/                          # Instruments, Web Desktop, REST, MCP, USB
```

---

## Key Technical Decisions

### 1. The `BoardProfile` Trait Contract

```rust
pub trait BoardProfile: Send + Sync + 'static {
    const BOARD_ID: &'static str;
    const NAME: &'static str;
    const MCU_PACKAGE: McuPackage; // QFN-60 (RP2350A) or QFN-80 (RP2350B)
    const GPIO_COUNT: u32;         // 30 or 48

    // Pin Zone Definitions
    fn programmer_pins() -> ProgrammerPins;
    fn adapter_pins() -> AdapterPins;
    fn flexible_pool() -> core::ops::RangeInclusive<u32>;
    fn analog_frontend() -> AnalogFrontendConfig;

    // Feature Flags
    fn features() -> BoardFeatures;
}
```

### 2. Extensible Analog Front-End Modeling

```rust
pub enum AnalogFrontendConfig {
    /// Baseline: Direct pin routing into RP2350 SAR ADC (e.g. Pico 2)
    Direct {
        adc_pins: &'static [u32],       // e.g. [26, 27, 28, 29]
        adc_channels: &'static [u8],    // e.g. [0, 1, 2, 3]
        attenuation: u32,               // 1x
        dual_personality: bool,         // True: shared with digital logic analyzer
    },

    /// High-Precision: SPI-controlled Programmable Gain Amplifier (e.g. TI PGA113)
    Pga113 {
        adc_pin: u32,                   // Direct connection to ADC0
        cs_pin: u32,                    // SPI Chip Select
        sck_pin: u32,                   // SPI Clock
        mosi_pin: u32,                  // SPI Data In
        gains: &'static [u16],          // [1, 2, 5, 10, 20, 50, 100, 200]
        has_vcal: bool,                 // Built-in auto-calibration channel
    },
}
```

### 3. Decoupling the Core Firmware (`firmware-rs`)

`firmware-rs` no longer hardcodes pin numbers. When initializing instruments:
- **Programmer**: Configures SWD/SWIO on `Board::programmer_pins()`.
- **Logic Analyzer**: Verifies capture span against `Board::flexible_pool()`.
- **Oscilloscope**: Initializes either direct ADC sampling or communicates with the PGA113 via `agentla_bsp::drivers::pga113`.
- **Manifest Endpoint (`/api/board/manifest`)**: Dynamically serializes the active `BoardProfile` into JSON.

---

## Risks / Trade-offs & Mitigations

| Risk | Mitigation |
|---|---|
| **Compilation Overhead across Workspace**: Multiple crates could increase build time. | `agentla-bsp` is lightweight (`no_std`) with zero heavy dependencies; incremental builds remain fast. |
| **Breaking Baseline Pico 2**: Refactoring could introduce regressions on existing hardware. | `boards/pico2` implements the exact current pinout (GPIO 0–9 programmer, 10–25 flex, 26–29 ADC); verified on live probe before merging. |
| **PGA113 SPI Contention**: PGA113 SPI bus sharing could collide with target SPI bridge. | PGA113 uses dedicated CS pin and independent SPI transaction lock in `agentla-bsp`. |
