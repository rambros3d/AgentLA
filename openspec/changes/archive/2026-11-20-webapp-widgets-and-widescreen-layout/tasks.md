## 1. Widget Micro-Libraries Integration

- [x] 1.1 Stage `pureknob.min.js` (Canvas2D rotary knobs, endless encoders, radial gauges) into `webui/`
- [x] 1.2 Stage `nipplejs.min.js` (2D virtual analog joystick and XY touchpad) into `webui/`
- [x] 1.3 Stage `ogl.min.js` (Minimalist WebGL 3D engine with native Quaternion and Euler math) into `webui/`
- [x] 1.4 Implement `agentla-led-view.js` (Neopixel 1D strip, 2D matrix, and ring visualizer with bloom & power telemetry) into `webui/`
- [x] 1.5 Implement `agentla-servo-view.js` (Parametric servo horn visualizer & timeline keyframe sequencer) into `webui/`
- [x] 1.6 Implement `agentla-thermal-view.js` (Bilinear interpolation thermal heatmap engine for AMG8833/MLX90640) into `webui/`
- [x] 1.7 Implement `agentla-pinout-view.js` (Interactive 40-pin RP2350 arbiter header visualizer) into `webui/`
- [x] 1.8 Implement `agentla-storage-map.js` (LittleFS flash sector grid wear visualizer) into `webui/`

## 2. Firmware HTTP Route Registration & Packaging

- [x] 2.1 Register GET routes in `firmware-rs/src/web.rs` for `pureknob.min.js`, `nipplejs.min.js`, `ogl.min.js`, and `agentla-*.js`
- [x] 2.2 Verify `tools/pack_webui.py` discovers, gzips, and packages all new assets within the 1 MB partition headroom

## 3. Web Application Widescreen Multi-Column Overhaul

- [x] 3.1 Refactor `webui/barefoc.html` for 12-column widescreen layout (`lg:grid-cols-12`), integrating `pure-knob` target setpoint dial and `nipplejs` 2-DOF gimbal joystick
- [x] 3.2 Refactor `webui/servo.html` for 12-column widescreen layout, integrating `agentla-servo-view.js` parametric horn simulator and channel angle dials
- [x] 3.3 Refactor `webui/neopixel.html` for 12-column widescreen layout, integrating `agentla-led-view.js` with matrix/ring support and side-by-side pattern controls
- [x] 3.4 Refactor `webui/sensor.html` for 12-column widescreen layout, integrating `ogl.min.js` for 3D IMU quaternion attitude visualization and radial telemetry gauges
- [x] 3.5 Refactor `webui/files.html` for 12-column widescreen layout, integrating `agentla-storage-map.js` flash block wear visualizer

## 4. Documentation & Developer Skills

- [x] 4.1 Update `SKILLS/agentla-webapp/SKILL.md` to document the new widget suite, copy-paste snippets, and widescreen multi-column guidelines

## 5. Verification & Live Deployment

- [x] 5.1 Run pytest test suites (`test_webui_assets.py`, `test_mcu_autonomous_testing.py`) and assert zero emojis, zero CSS dot indicators, and asset integrity
- [x] 5.2 Build release firmware binary, package OTA container, and deploy WebUI update to the live Pi Pico 2 target (`10.10.10.10`)
