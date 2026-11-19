## Why

BareFOC Studio in AgentLA (`webui/barefoc.html`) currently resembles an ODrive GUI clone, which obscures BareFOC and SimpleFOC native features (Clarke/Park transforms, SVPWM sector modulation, cascaded position-velocity-torque loop tuning, 2-DOF differential kinematics, non-linear transmission LUTs, MultiBus register maps, and ASCII Commander terminal). Additionally, hand-rolled HTML5 2D canvas rolling charts lack interactive zoom, pan, hover cursors, and multi-channel scaling.

Overhauling BareFOC Studio to a BareFOC-first design using DaisyUI 5 semantic components and integrating uPlot will deliver a responsive, professional motor-control workbench optimized for embedded robotics.

## What Changes

- Bundle `uplot.min.js` and `uplot.min.css` into `webui/` as a high-performance timeseries charting engine for all WebUI oscilloscopes.
- Overhaul `webui/barefoc.html` using DaisyUI 5 semantic components (`tab`, `card`, `join`, `kbd`, `badge`, `radial-progress`, `modal`, `drawer`):
  - **Header & Topology Control**: Replace generic status banner with a hardware topology selector pill (4x 1-DOF, 2x 2-DOF, 1x 1-DOF + 1x 2-DOF, 2x 1-DOF + 1x 2-DOF), MultiBus address selector, and motor tabs (M0..M3) with surface styling (no CSS dot indicators).
  - **FOC Vector Scope**: Real-time Clarke/Park vector polar locus ($I_\alpha, I_\beta$ trajectory, $I_d, I_q$ vector circle) and SVPWM space vector hexagon displaying active sector and voltage modulation circle.
  - **Interactive Cascaded Control Loop Diagram**: Visual signal flow schematic (Target -> Outer Joint PID -> P-Angle -> PID-Velocity -> Current DQ -> SVPWM -> Inverter -> Motor -> Sensor) with inline editable gain pills.
  - **uPlot Multi-Channel Strip Charts**:
    - Dual/Quad-trace oscilloscope for setpoint vs feedback ($\theta, \omega, I_q$).
    - Motor 3-phase current trace ($I_a, I_b, I_c$).
    - Outer joint trajectory trace ($\theta_j$ setpoint vs measured).
  - **Kinematics & Multi-Motor Workbench**:
    - 2-DOF Differential Gimbal pitch/roll interactive orientation display.
    - Non-linear transmission LUT curve visualizer and spline editor with burst I2C writing.
    - Calibration status monitor and autonomous/interactive calibration trigger buttons.
  - **MultiBus Register Table**: Interactive register map table with live polling and direct inline hex/float writes.
  - **SimpleFOC Commander Terminal**: Interactive ASCII stream console with syntax highlighting, command history, and shortcuts.

## Capabilities

### New Capabilities
- `uplot-telemetry`: High-performance timeseries charting library integration for real-time WebUI oscilloscopes and strip charts.

### Modified Capabilities
- `barefoc-studio-application`: Overhaul BareFOC Studio UI/UX with DaisyUI 5, polar vector scope, SVPWM hexagon, cascaded control block diagram, and kinematics workbench.

## Impact

- `webui/uplot.min.js`, `webui/uplot.min.css`: Added bundled dependencies for zero-overhead charting.
- `webui/barefoc.html`: Complete UI/UX redesign conforming to DaisyUI 5 and AgentLA design standards.
- Zero breaking changes to `agentla-bridge.js` or underlying HTTP/WebSocket endpoints.
