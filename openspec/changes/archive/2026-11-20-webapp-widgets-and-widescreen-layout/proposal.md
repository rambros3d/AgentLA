## Why

AgentLA web applications currently suffer from cramped, vertically-stacked layouts on wide-screen displays, leaving horizontal screen real estate underutilized. Furthermore, existing interactive controls (rotary dials, servo horns, joysticks, and 3D attitude indicators) are either implemented ad-hoc or missing, making it difficult for user apps to present rich physical hardware state without bundling heavy external dependencies like Three.js (>650 KB) or Chart.js.

Standardizing a suite of ultra-lightweight (<30 KB gzip total), zero-dependency, Canvas2D/WebGL-driven micro-libraries—specifically `pure-knob` (rotary dials/gauges), `nipplejs` (2D virtual joysticks), `OGL` (minimalist WebGL 3D/quaternion engine), and domain-specific visualizers (Neopixel LED engine, Servo horn kinematics, Thermal camera heatmaps, RP2350 pinout, and LittleFS storage maps)—and refactoring system apps to adopt responsive multi-column layouts will transform AgentLA into a high-density, desktop-class engineering bench.

## What Changes

- Add canonical widget micro-libraries to `webui/` as shared in-ROM assets:
  - `pureknob.min.js`: Zero-dependency Canvas2D rotary knobs, endless encoders, radial gauges, and double-click numeric entry (~3.8 KB gzip).
  - `nipplejs.min.js`: Multi-touch 2D virtual analog joystick and XY touchpad (~3.2 KB gzip).
  - `ogl.min.js`: Minimalist WebGL 3D engine with native Quaternion (`Quat`) and Euler math (~14.5 KB gzip).
  - `agentla-led-view.js`: Reusable 1D strip, 2D matrix (serpentine/zigzag), and circular ring Neopixel visualizer with radial bloom shaders and live power dissipation ($mA$) calculation.
  - `agentla-servo-view.js`: Reusable parametric servo horn visualizer (single arm, double arm, cross, winch drum) with travel arcs ($0\text{--}180^\circ$) and keyframe timeline sequencer.
  - `agentla-thermal-view.js`: Reusable bilinear interpolation heatmap renderer for thermal cameras (AMG8833 $8\times 8$, MLX90640 $32\times 24$) with Ironbow and Inferno palettes.
  - `agentla-pinout-view.js`: Interactive 40-pin RP2350 header visualizer with live Pin Arbiter ownership badges.
  - `agentla-storage-map.js`: LittleFS flash sector grid visualizer ($440\times 4\text{ KB}$ user blocks, $256\times 4\text{ KB}$ system blocks) with wear-leveling counts.
- Register new asset HTTP routes in `firmware-rs/src/web.rs`.
- Overhaul web application viewports for widescreen multi-column layouts (`grid grid-cols-1 lg:grid-cols-12` or multi-column flex decks):
  - `webui/barefoc.html`: Multi-column FOC cockpit with `pure-knob` target setpoint dial, `nipplejs` 2-DOF gimbal joystick, FOC vector scope, and side-by-side telemetry HUD.
  - `webui/servo.html`: Multi-column actuation deck with `agentla-servo-view` parametric horns, multi-channel pose timeline sequencer, and gamepad/joystick integration.
  - `webui/neopixel.html`: Multi-column lighting studio with `agentla-led-view` 1D/2D/ring simulator, zoom/pan canvas, 60 FPS shaders, and power telemetry.
  - `webui/sensor.html`: Multi-column sensor studio with `OGL` 3D IMU attitude viewer (quaternion-driven rotating PCB/cube), `pure-knob` radial meters, and live uPlot sensor streams.
  - `webui/files.html`: Multi-column file manager with `agentla-storage-map` flash wear visualizer and bento file explorer.
- Update `SKILLS/agentla-webapp/SKILL.md` to document the new widget suite and provide copy-paste boilerplates.
- Strictly maintain zero emojis, zero CSS dot indicators, bottom dock geometry, and `#![no_std]` firmware constraints.

## Capabilities

### New Capabilities
- `widget-library-suite`: Canonical in-ROM widget micro-libraries (`pureknob.min.js`, `nipplejs.min.js`, `ogl.min.js`, `agentla-led-view.js`, `agentla-servo-view.js`, `agentla-thermal-view.js`, `agentla-pinout-view.js`, `agentla-storage-map.js`) served with gzip compression and immutable cache headers.
- `widescreen-multicolumn-layout`: Standardized responsive multi-column grid architecture across all AgentLA system web applications on landscape and widescreen viewports.

### Modified Capabilities
- `native-barefoc-system-app`: Integrates `pure-knob` setpoint dials, `nipplejs` 2-DOF gimbal joystick, and full-bleed multi-column widescreen layout.
- `native-servo-system-app`: Integrates `agentla-servo-view` parametric horn simulator, angle dials, and responsive widescreen layout.
- `native-neopixel-system-app`: Integrates `agentla-led-view` standardized component, matrix/ring visualizers, and responsive widescreen layout.
- `native-sensor-studio`: Integrates `OGL` 3D attitude viewer for IMU quaternion orientation, `pure-knob` radial telemetry gauges, and responsive widescreen layout.

## Impact

- `webui/`: New shared JS assets (`pureknob.min.js`, `nipplejs.min.js`, `ogl.min.js`, `agentla-*.js`); updated HTML/JS files (`barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`, `files.html`).
- `firmware-rs/src/web.rs`: Additional HTTP static file route registrations for the new shared assets.
- `SKILLS/agentla-webapp/SKILL.md`: Expanded developer documentation and usage examples.
- `build/webui.tar`: Total compressed size increases by ~25-35 KB, remaining well under the 1 MB `SYSTEM_FS` partition limit (>580 KB headroom).
- `test/test_webui_assets.py`: Additional assertions verifying new asset integrity, zero-emoji compliance, and multi-column widescreen layouts.
