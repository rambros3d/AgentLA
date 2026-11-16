## Context

BareFOC is a `#![no_std]` Rust implementation of Field Oriented Control targeting embedded microcontrollers (STM32, ESP32, CH32) with support for 3-phase BLDC, stepper, and brushed DC motors. It features three protocol implementations:
1. `barefoc-fibre`: ODrive-compatible binary RPC protocol with self-describing JSON schema.
2. `barefoc-commander`: Upstream SimpleFOC-compatible single-character/ASCII protocol.
3. `barefoc-multibus`: Dynamixel/Miniata-style descriptor-driven register table transport.

AgentLA is an embedded workbench and instrumentation platform running on RP2040/RP2350 microcontrollers. It contains an operating system shell with an App Store, LittleFS file system, streaming tarball unpacker, and hardware bridges (UART `/api/v1/terminal/ws`, CAN `/api/v1/can/ws`, power rail telemetry, and emergency safe-state pin release).

This design establishes a native, user-installable application, `BareFOC Studio`, running within AgentLA's WebUI and LittleFS storage ecosystem.

## Goals / Non-Goals

**Goals:**
- Provide a single, self-contained single-page application (`apps/barefoc/index.html`) unpackable into LittleFS and under 100 KB total size.
- Deliver an ODrive GUI-grade experience with 5 core views:
  1. Live Studio & Multi-Mode Dashboard.
  2. Cascaded Controller & PID/LPF Tuner.
  3. 60 FPS Dual-Trace Oscilloscope & Step Response Analyzer.
  4. Motor & Sensor Setup / Alignment Wizard.
  5. Low-Level Protocol & Register Console.
- Support all three BareFOC protocol lanes seamlessly via a unified client adapter interface.
- Integrate with `agentla-bridge.js` for hardware safe-state (`AgentLA.emergencySafeState()`), target power rail control, and hardware UART/CAN WebSocket bridges.
- Provide a standalone virtual motor simulator (`?demo=1`) for testing and preview without live hardware.
- Package as `apps/packages/barefoc.tar` and register in `apps/catalog.json`.
- Strictly adhere to AgentLA standards: zero emojis, no CSS dot indicators, full-bleed viewport layout, and DaisyUI theming.

**Non-Goals:**
- Modifying AgentLA core firmware or changing LittleFS partition boundaries.
- Re-implementing heavy server-side Python toolchains (e.g. odrivetool); all Fibre framing and schema decoding is handled client-side in pure JavaScript.

## Decisions

### D1: Self-Contained Single-File Application Architecture
- **Decision**: Package the application as a single self-contained `index.html` referencing shared system assets (`/daisyui.css`, `/style.css`, `/themes.css`, `/agentla-bridge.js`).
- **Rationale**: Keeps total package size under 80 KB (far below the 768 KB limit), ensures instantaneous loading from SPI flash, and eliminates complex multi-file asset resolution in LittleFS.
- **Alternatives Considered**: Multi-file bundle with Webpack/Vite; rejected due to build complexity and unnecessary tarball bloat.

### D2: Unified Protocol Adapter Layer
- **Decision**: Abstract device interactions behind a unified `FOCDevice` interface with three concrete adapters: `FibreAdapter`, `CommanderAdapter`, and `MultiBusAdapter`.
- **Rationale**: Allows the user to switch between a native ODrive board, a SimpleFOC serial target, or a MultiBus I2C/SPI actuator without changing UI components or plotting logic.
- **Alternatives Considered**: Separate app instances per protocol; rejected because users often operate mixed setups or transition between protocols on the same motor.

### D3: Zero-Dependency Canvas 2D Oscilloscope
- **Decision**: Implement the multi-trace real-time plotter using a custom HTML5 Canvas 2D ring buffer (`Float32Array`) running on `requestAnimationFrame`.
- **Rationale**: Eliminates dependencies on Chart.js or Plotly, consumes less than 100 KB RAM in browser memory, and easily maintains 60 FPS update rates during fast step-response transients.
- **Alternatives Considered**: SVG-based graphing (too slow for high-rate FOC telemetry) or Chart.js (adds 150+ KB of script overhead).

### D4: Direct Integration with AgentLA Safety & Power Systems
- **Decision**: Connect UI emergency controls directly to `AgentLA.emergencySafeState()` and `AgentLA.power.setEnabled()`.
- **Rationale**: In FOC motor tuning, runaway or inverter desaturation can cause hardware damage within milliseconds. Hardware-level pin high-Z and power isolation are critical.

## Risks / Trade-offs

- **[Risk] High-rate telemetry saturating UART bridge WebSocket**
  - **Mitigation**: Implement client-side frame decimation and adjustable telemetry polling rates (10 Hz, 20 Hz, 50 Hz, 100 Hz).
- **[Risk] Target disconnect or baud rate mismatch**
  - **Mitigation**: Visual connection status indicator in top toolbar with auto-reconnect fallback and clear error reporting.
- **[Risk] Complex Fibre schema payloads overflowing memory**
  - **Mitigation**: Minimalist streaming JSON parser for endpoint 0 schema that retains only motor/controller fields.
