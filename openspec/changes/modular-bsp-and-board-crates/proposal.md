## Why

AgentLA is evolving from a single development board (Raspberry Pi Pico 2) into a multi-board hardware ecosystem that includes custom instrumentation carrier boards (such as BakuProbe / AgentProbe with RP2350B, 48 GPIOs, level shifters, current sense, and programmable gain amplifiers). Today, pin mappings, zone boundaries, and analog front-end details are partially hardcoded in `firmware-rs`. Adding a new board or custom user hardware currently requires modifying core firmware files, introducing high regression risks and conditional compilation clutter.

Adopting an idiomatic embedded Rust **Board Support Package (BSP) Workspace architecture** decouples hardware definitions from application firmware. Dedicated board crates define pin mappings and peripheral configurations in complete isolation, while a shared `agentla-bsp` library houses reusable external IC drivers (like the TI PGA113 amplifier and TI INA180 current sense IC).

---

## What Changes

1. **Shared BSP Crate (`crates/agentla-bsp`)**:
   - Defines the core `BoardProfile` trait and hardware zone types (`ProgrammerPins`, `AdapterPins`, `FlexiblePool`, `AnalogFrontend`).
   - Houses reusable, hardware-agnostic external IC drivers:
     - `pga113`: TI PGA113 Zero-Drift Programmable Gain Amplifier driver (1x to 200x gains over SPI).
     - `ina180`: TI INA180 Current Sense Amplifier telemetry helper.
     - `tcan334`: Differential CAN-FD transceiver control with switchable $120\,\Omega$ termination.
     - `txs0108`: Active level-shifter power rail management.

2. **Dedicated Board Crates (`boards/*`)**:
   - **`boards/pico2`**: Raspberry Pi Pico 2 baseline board crate (RP2350A, GPIO 0–29, direct ADC).
   - **`boards/bakuprobe`**: BakuProbe instrument board crate (RP2350B, GPIO 0–47, PGA113 front-end, INA180 current sense, level-shifted headers).
   - **`boards/templates`**: Boilerplate template crate allowing third-party developers or users to define custom PCBs without touching core AgentLA code.

3. **Core Firmware Decoupling (`firmware-rs`)**:
   - `firmware-rs` depends on the selected board crate via Cargo workspace configuration or features.
   - Core instrument engines (Logic Analyzer, Oscilloscope, Bus Bridges, REST API, CMSIS-DAP) query the active `BoardProfile` dynamically.
   - `/api/board/manifest` is generated automatically from the active board profile and its metadata.

---

## Capabilities

### New Capabilities
- `modular-bsp-architecture`: Common `BoardProfile` trait, pin zone types, and reusable external IC drivers (`pga113`, `ina180`, `tcan334`) in a shared library crate.
- `board-crates-workspace`: Separate board crates (`boards/pico2`, `boards/bakuprobe`, `boards/templates`) implementing `BoardProfile` in complete compilation isolation.

### Modified Capabilities
- `dynamic-board-manifest`: Board manifest `/api/board/manifest` and pin matrix metadata are dynamically generated from the active board crate rather than a single hardcoded JSON file.

---

## Impact

- **Zero Breaking Changes to Host Tools**: REST endpoints, USB CDC-ACM, CMSIS-DAP, and MCP server interfaces remain identical.
- **Compilation Safety**: Pin assignment conflicts (e.g., overlapping pins or invalid ADC channel bindings) are caught at compile-time within the board crate.
- **Third-Party Extensibility**: Community and enterprise users can develop custom carrier boards in private repositories simply by importing `agentla-bsp`.
