## Why

BareFOC is a comprehensive `#![no_std]` Rust Field-Oriented Control (FOC) framework for BLDC and stepper motors supporting multiple protocol lanes (Fibre RPC, SimpleFOC Commander ASCII, and MultiBus register map). However, developers currently lack an integrated, user-installable graphical interface within AgentLA to configure, tune, calibrate, and visualize motor performance in real time. 

Providing a native, installable BareFOC Studio app on AgentLA bridges this gap by packaging an ODrive GUI-grade motor studio—featuring cascaded PID/LPF tuning, a 60 FPS dual-trace oscilloscope, an automated alignment wizard, and low-level protocol diagnostics—directly into AgentLA's LittleFS application ecosystem.

## What Changes

- Add a new user-installable application package `apps/barefoc/` containing:
  - `app.json`: Application metadata, categories, permissions (`uart`, `can`, `gpio_flexible`), and version info.
  - `icon.svg`: Crisp, technical vector icon for desktop launcher and bottom dock.
  - `index.html`: Feature-complete single-page application with 5 functional views (Live Studio, Controller Tuning, Oscilloscope, Hardware Wizard, Low-Level Console).
- Register BareFOC Studio in `apps/catalog.json` with package metadata, tags, and dependencies.
- Provide pre-packaged `apps/packages/barefoc.tar` archive for 1-click LittleFS installation from AgentLA App Store.
- Implement pure JavaScript client adapters for:
  - **Fibre RPC**: Binary framing, endpoint 0 JSON schema introspection, typed read/write, and heartbeat.
  - **SimpleFOC Commander**: ASCII command formatting (`M1 V 10`, `M1 C 2`, `?`), response parser, and monitor stream decoder.
  - **MultiBus**: Register map table reader and writer (Miniata/Dynamixel-style control table).
- Integrate with `agentla-bridge.js` for target power telemetry, emergency safe-state triggers, and hardware UART/CAN bridges.
- Provide a built-in virtual motor simulation mode for standalone / demo evaluation (`?demo=1`).

## Capabilities

### New Capabilities
- `barefoc-studio-application`: User-installable BareFOC Studio application on AgentLA featuring multi-protocol communications (Fibre, Commander, MultiBus), real-time cascaded PID tuning, 60 FPS dual-trace canvas oscilloscope, motor/sensor alignment wizard, and hardware safety controls.

### Modified Capabilities
<!-- No existing capability requirements are changing -->

## Impact

- `apps/barefoc/`: New application directory added.
- `apps/catalog.json`: Updated with `barefoc` catalog entry.
- `apps/packages/barefoc.tar`: Generated package tarball for LittleFS installation.
- Firmware: Zero changes required to core firmware; leverages existing LittleFS tar unpacker and `/api/v1/terminal/ws`, `/api/v1/can/ws`, and `agentla-bridge.js` endpoints.
