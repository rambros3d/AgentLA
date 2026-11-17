## Context

Developers testing I2C and SPI breakout boards currently face a tedious trial-and-error bring-up cycle requiring custom code, toolchains, and serial debugging. AgentLA provides physical bus masters in `firmware-rs/src/bridge/` (`i2c.rs` and `spi.rs`) with low-level REST APIs (`/api/v1/bridge/i2c/scan`, `/api/v1/bridge/i2c/transfer`, `/api/v1/bridge/spi/transfer`). However, there is no visual, in-ROM instrument for discovering devices, exploring register layouts, and streaming live sensor telemetry.

This design introduces **Sensor Studio** at `webui/sensor.html` (accessible at `/sensor`, `/sensors`, `/bus`), featuring dual-bus scanning, chip fingerprinting, interactive 256-byte register hex maps, and specialized visualizers. It also equips the MCP server with direct sensor exploration and register inspection tools.

## Goals / Non-Goals

**Goals:**
- Provide `webui/sensor.html` as a permanent system asset in `SYSTEM_FS` (~18 KB gzipped).
- Register routes `/sensor`, `/sensors`, and `/bus` in `firmware-rs/src/web.rs` utilizing `SystemWebFile`.
- Add **Sensor** to the top navigation header across all system views (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`).
- Add `dock-app-sensor` in `webui/agentla-bridge.js` and `webui/app.js` using Solar **Bold Duotone Radar** (`ref/solar-icons/packages/core/svgs/map/BoldDuotone/radar-2.svg`).
- Implement dual-bus frontend logic:
  - **I2C mode**: 128-address scan grid, speed selection (100k, 400k, 1M), pull-up error warnings.
  - **SPI mode**: Pin routing (SCK, MOSI, MISO, CS0-CS3), Mode selection (0, 1, 2, 3), speed scaling (1M to 20M), and JEDEC / WHO_AM_I heuristic probing.
- Implement sensor profile database:
  - MPU6050/9250 (IMU), TMAG5273 (3D Hall/Angle), AS5600 (Angle), BME280/BMP280 (Climate), INA219/226 (Power), LIS3DH, VL53L0X.
- Implement pure Canvas visualizers without heavy external libraries:
  - `imu_3d` (3D attitude prism)
  - `angle_dial` (rotary compass gauge)
  - `env_dashboard` (climate dials)
  - `power_monitor` (V/I/P meters)
  - `generic_stream` (scrolling multi-trace plotter)
- Expose MCP tools (`sensor_scan`, `sensor_read_reg`, `sensor_write_reg`, `sensor_poll`) in `crates/agentla-mcp` and `firmware-rs/src/mcp.rs`.
- Maintain strict `#![no_std]`, no CSS dot indicators, and zero emoji standards.

**Non-Goals:**
- On-device sensor fusion (Madgwick/Mahony filter math is computed in the frontend browser canvas).
- Altering the 512 KB `SYSTEM_FS` partition boundaries.

## Decisions

### D1: Pure HTML5 Canvas for 3D Visualizer
- **Decision**: Implement the 3D attitude cube in ~120 lines of pure HTML5 2D Canvas matrix math rather than importing Three.js or WebGL engines.
- **Rationale**: Keeps the compressed asset under 20 KB to strictly protect the 512 KB `SYSTEM_FS` budget while delivering smooth 60 FPS rotation.

### D2: Automatic SPI Discovery via Heuristic Register Probes
- **Decision**: On SPI bus probe, query JEDEC ID (`0x9F`) and common register signatures (`0x80 | reg`) across active CS pins.
- **Rationale**: Solves the SPI lack-of-addressing problem, enabling automatic identification of SPI NOR flash and popular SPI sensors.

### D3: Iconography
- **Decision**: Use Solar **Bold Duotone Radar** (`radar-2.svg`) with `currentColor` and `opacity="0.5"`.
- **Rationale**: Concentric scanning rings and sweeping radar beam directly convey active bus exploration.

## Risks / Trade-offs

- **[Risk] Partition budget exhaustion**
  - **Mitigation**: `sensor.html` is ~18 KB gzipped. Total system assets remain under 430 KB, well within the 512 KB partition cap.
- **[Risk] Bus hanging on unpopulated/stuck lines**
  - **Mitigation**: `firmware-rs` I2C hardware drivers enforce 10 ms hardware timeouts on clock stretching.
