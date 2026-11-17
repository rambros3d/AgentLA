## 1. Web Asset Creation & Dual-Bus UI

- [ ] 1.1 Create `webui/sensor.html` with Bento layout and responsive DaisyUI styling
- [ ] 1.2 Implement dual-bus mode switcher (I2C radar matrix vs SPI CS probe panel)
- [ ] 1.3 Implement 128-address visual scan radar (`0x08..0x77`) with ping animation
- [ ] 1.4 Build chip fingerprinting registry (MPU6050, TMAG5273, BME280, AS5600, INA219, etc.)
- [ ] 1.5 Implement 256-byte interactive register map with bitfield toggles and dirty-byte live highlighting
- [ ] 1.6 Implement pure Canvas visualizers (`imu_3d`, `angle_dial`, `env_dashboard`, `power_monitor`, `generic_stream`)
- [ ] 1.7 Add canonical top-rail navigation to `webui/sensor.html`

## 2. Navigation Rail, Dock & Deep Linking

- [ ] 2.1 Add Sensor navigation link to `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, `webui/barefoc.html`, `webui/servo.html`, and `webui/neopixel.html`
- [ ] 2.2 Formalize Sensor item in floating dock and application drawer in `webui/agentla-bridge.js` and `webui/app.js` using Solar bold duotone radar icon
- [ ] 2.3 Ensure surface-only active state highlights with zero CSS dot indicators
- [ ] 2.4 Configure deep linking and hash modes (`/sensor` and `#/sensor`)

## 3. Firmware Route Registration

- [ ] 3.1 Register `/sensor`, `/sensor/`, and `/sensor.html` in `firmware-rs/src/web.rs` backed by `SystemWebFile::new("sensor.html", "text/html")`
- [ ] 3.2 Register 301 redirect aliases for `/sensors` and `/bus` in `firmware-rs/src/web.rs` pointing to `/sensor`
- [ ] 3.3 Verify `firmware-rs` compiles cleanly `#![no_std]` targeting `thumbv8m.main-none-eabihf`

## 4. MCP Tool Declaration & Dispatch

- [ ] 4.1 Declare `TOOL_SENSOR_SCAN`, `TOOL_SENSOR_READ_REG`, `TOOL_SENSOR_WRITE_REG`, and `TOOL_SENSOR_POLL` in `crates/agentla-mcp/src/tools.rs`
- [ ] 4.2 Add tools to `ALL_TOOLS` array in `crates/agentla-mcp/src/tools.rs`
- [ ] 4.3 Implement JSON-RPC tool dispatch in `firmware-rs/src/mcp.rs` delegating to `crate::bridge::i2c` and `crate::bridge::spi`
- [ ] 4.4 Update `webui/mcp.json` tool count
- [ ] 4.5 Verify `cargo test -p agentla-mcp` passes cleanly

## 5. Packaging & Verification

- [ ] 5.1 Update `tools/pack_webui.py` to include `sensor` in `SYSTEM_APPS` and package `sensor.html` into `build/webui.img` and `build/webui.tar`
- [ ] 5.2 Validate asset size within `SYSTEM_FS` partition headroom
- [ ] 5.3 Add automated pytest tests verifying routes, dock handlers, zero emojis, and MCP tool discovery
- [ ] 5.4 Run all test suites to confirm clean execution
