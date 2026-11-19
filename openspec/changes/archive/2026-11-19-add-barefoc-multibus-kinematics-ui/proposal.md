## Why

`AgentLA-rust` features a WebUI BareFOC control tool in `webui/barefoc.html` communicating over I2C/SPI via `MultiBusAdapter`. While basic single-motor telemetry and PID gains are currently supported, the interface lacks multi-motor selection, robotic kinematics parameter tuning (gear ratio, backlash, compliance, 2-DOF outer PIDs), non-linear LUT curve visualization, and on-device calibration control.

Updating `webui/barefoc.html` provides complete host-side control and observability over BareFOC actuators without requiring firmware reflashing or external Python tools.

## What Changes

- Add a Multi-Motor tab selector to `webui/barefoc.html` allowing operators to switch between attached motors ($0..N-1$) via `MotorSelect` (`0x70`).
- Add a Kinematics Panel:
  - Sliders and numeric inputs for `GearRatio` (`0x81`), `BacklashDelta` (`0x82`), `OmegaThresh` (`0x83`), and `TorsionalStiffness` (`0x84`).
  - Dual 2D joint angle display dials (Pitch $\alpha$ and Roll $\beta$) and joint setpoint targets.
  - Dual outer position PID parameter sliders (`OuterPidAlpha_*` and `OuterPidBeta_*`).
  - 2x2 differential coupling matrix configuration table (`0x9A..0x9D`).
- Add an interactive non-linear LUT Curve Editor with a canvas for drawing or previewing cam/tendon curves, real-time monotonicity validation, and single-burst write streaming.
- Add an On-Device Calibration & Flash Storage panel with triggers for autonomous limit sweeps, interactive homing, and Save to Flash / Load from Flash with CRC integrity status display.
- Strict adherence to AgentLA design standards: zero emojis, no CSS dot indicators, and full-bleed viewport dock geometry.

## Capabilities

### New Capabilities
- `barefoc-kinematics-ui`: WebUI controls, multi-motor switching, and interactive kinematics/calibration tools in `webui/barefoc.html`.

## Impact

- `AgentLA-rust`: `webui/barefoc.html`.
