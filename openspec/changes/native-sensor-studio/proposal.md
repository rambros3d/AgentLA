## Why

Bringing up new I2C or SPI breakout boards (IMUs, magnetic encoders, barometers, environmental sensors, current monitors) currently requires writing temporary firmware sketches, compiling, flashing, and parsing serial debug prints. If communication fails, diagnosing whether the failure is electrical, addressing, or protocol-related is slow and error-prone.

Furthermore, autonomous AI agents interacting with AgentLA via MCP currently lack unified sensor discovery and register inspection tools to verify circuit connectivity and read sensor telemetry automatically.

Introducing **Sensor Studio** as a native in-ROM system instrument (`/sensor`) provides instant, zero-compile hardware bring-up: scanning buses, fingerprinting silicon signatures, rendering interactive register maps with bitfield editors, and streaming live visual telemetry (3D attitude cubes, angle dials, climate meters). Adding companion MCP tools empowers AI agents to autonomously audit and debug attached sensors.

## What Changes

- Add `webui/sensor.html`: Native dual-bus (I2C + SPI) workbench interface in `SYSTEM_FS`.
- Register firmware routes in `firmware-rs/src/web.rs`:
  - Route `/sensor` to `SystemWebFile::new("sensor.html", "text/html")`.
  - Add redirect aliases `/sensors` and `/bus` targeting `/sensor`.
- Integrate into AgentLA OS Navigation & Shell:
  - Add **Sensor** to the top navigation header across all canonical pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`).
  - Add dock launcher `dock-app-sensor` using the Solar icon **Bold Duotone Radar** (`ref/solar-icons/packages/core/svgs/map/BoldDuotone/radar-2.svg`).
  - Configure deep linking for `/sensor` and `#/sensor`.
- Dual-Bus Engine:
  - **I2C**: 128-address visual scan radar (`0x08..0x77`), repeated-start register read/write, bus speed switching (100k, 400k, 1M).
  - **SPI**: Multi-CS pin management, SPI Modes (0, 1, 2, 3), speed scaling (1M to 20M), and JEDEC / WHO_AM_I heuristic probing.
- Sensor Profile Registry & Visualizers:
  - Built-in signature database (MPU6050/9250, TMAG5273, BME280, AS5600, INA219/226, LIS3DH, VL53L0X).
  - 256-byte interactive register map with bitfield toggles and dirty-byte live highlighting.
  - Specialized Canvas visualizers: `imu_3d` (3D attitude prism), `angle_dial` (rotary compass gauge), `env_dashboard` (climate dials), `power_monitor` (V/I/P meters), and `generic_stream` (multi-trace chart).
- MCP Autonomous Agent Tooling:
  - Add `sensor_scan`: Probe I2C/SPI buses and return detected chip identities.
  - Add `sensor_read_reg`: Read 1 to 32 bytes from device registers.
  - Add `sensor_write_reg`: Write bytes to device registers.
  - Add `sensor_poll`: High-rate batch register telemetry extraction.
  - Update `webui/mcp.json` tool count.
- Update `tools/pack_webui.py` to bundle `sensor.html` into `SYSTEM_FS`.

## Capabilities

### New Capabilities
- `native-sensor-studio`: Native in-ROM dual-bus (I2C + SPI) sensor discovery, register hex map, live telemetry plotter, and specialized visualizers routed at `/sensor` (with `/sensors` and `/bus` aliases).
- `mcp-sensor-tools`: Autonomous AI agent tools for I2C and SPI sensor discovery, register reading, register writing, and batch telemetry extraction.

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `webui/sensor.html`: Added as a permanent in-ROM system asset.
- `webui/agentla-bridge.js` & `webui/app.js`: Solar bold duotone radar SVG icon and dock button added.
- `firmware-rs/src/web.rs`: Routing table updated with `/sensor`, `/sensors`, and `/bus`.
- `crates/agentla-mcp` & `firmware-rs/src/mcp.rs`: New sensor tools added.
- `webui/mcp.json`: Tool count incremented.
- `SYSTEM_FS`: Packaged image size increases by ~18 KB gzipped (well within remaining margin).
