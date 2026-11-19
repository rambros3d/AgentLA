## Context

AgentLA's BareFOC Studio was originally scaffolded as an ODrive GUI clone, which focuses on ODrive axis states and bus dumps. This layout does not reflect BareFOC and SimpleFOC features:
1. BareFOC features explicit FOC transforms: Clarke ($I_a, I_b \rightarrow I_\alpha, I_\beta$), Park ($I_\alpha, I_\beta, \theta_e \rightarrow I_d, I_q$), and Space Vector PWM ($V_d, V_q \rightarrow V_\alpha, V_\beta \rightarrow T_a, T_b, T_c$). Engineers need to visually verify phase balances, rotor electrical angle alignment, and PWM saturation.
2. BareFOC supports advanced kinematics: 1-DOF actuators, 2-DOF coupled differential gimbals, non-linear transmission linkages, and spline LUTs.
3. The FishyServo board features 4 H-bridges, 4 current sense channels, and a TMAG5273 3D Hall sensor, operating across four runtime topologies (4x 1-DOF, 2x 2-DOF, 1x 1-DOF + 1x 2-DOF, 2x 1-DOF + 1x 2-DOF).
4. WebUI charts currently rely on hand-rolled `<canvas>` rendering without interactive inspection or multi-channel synchronization.

## Goals / Non-Goals

**Goals:**
- Bundle `uplot.min.js` and `uplot.min.css` (~40 KB) to provide 60 FPS, GPU-accelerated canvas timeseries charting with zooming, cursors, and legend toggling.
- Rebuild `webui/barefoc.html` with DaisyUI 5 semantic components, respecting strict AgentLA UI rules (zero emojis, surface styling active states without CSS dot indicators, full-bleed viewport `calc(100vh - 48px)`, bottom-dock anchoring).
- Build a dedicated **FOC Vector Scope**:
  - Clarke / Park polar locus display ($I_\alpha, I_\beta$ trajectory and $I_d, I_q$ vector).
  - Space Vector PWM (SVPWM) hexagon with sector boundaries (Sectors 1..6) and voltage circle.
- Build an **Interactive Cascaded Control Loop Block Diagram**:
  - Visual signal flow from Target $\rightarrow$ Outer Joint PID $\rightarrow$ P-Angle $\rightarrow$ PID-Velocity $\rightarrow$ Current DQ $\rightarrow$ SVPWM $\rightarrow$ Inverter $\rightarrow$ Motor $\rightarrow$ Sensor.
  - Interactive clickable nodes opening inline tuning inputs for $K_p, K_i, K_d$, ramp limits, and low-pass filters.
- Build a **Kinematics & Multi-Motor Workbench**:
  - Live topology switcher (4x 1-DOF, 2x 2-DOF, 1x 1-DOF + 1x 2-DOF, 2x 1-DOF + 1x 2-DOF) over MultiBus register `0x78` (`TopologyMode`).
  - 2-DOF Differential Gimbal pitch/roll interactive orientation visualizer.
  - Non-linear transmission LUT spline curve editor with drag-handles and streaming I2C burst writes (`0xA3..0xA4`).
  - Automated calibration wizard (`0xB0` CalibCommand triggers, `0xB1` CalibStatus, `0xB4` FlashStatus).
- Build a **MultiBus Register Explorer** and **SimpleFOC Commander Terminal**:
  - Live searchable register table with one-click read/write.
  - Terminal console with syntax highlighting, command history, and quick-action buttons.

**Non-Goals:**
- Modifying backend Rust firmware binaries or breaking existing HTTP/WebSocket protocol payloads.
- Adding heavyweight external chart bundles (Chart.js, D3, or Plotly).

## Decisions

### Decision 1: Adopt uPlot for high-frequency timeseries charts
- **Choice**: Bundle `uplot.min.js` and `uplot.min.css` into `webui/`.
- **Rationale**: uPlot is ~40 KB minified, has zero external dependencies, renders directly to 2D canvas, handles 150,000+ points at 60 FPS without memory leaks, and provides built-in sync across multiple plots, pan/zoom, and crosshairs.
- **Alternatives Considered**: Hand-rolled canvas (lacks cursor inspection, auto-scaling, and zoom), Chart.js (heavyweight ~250 KB, slow on high data rates), Plotly (multi-megabyte).

### Decision 2: Implement custom canvas visualizers for polar vector locus and SVPWM hexagon
- **Choice**: Use lightweight, dedicated HTML5 2D canvas renderers for non-timeseries visualizers:
  - Polar scope: $I_d, I_q$ vector and $I_\alpha, I_\beta$ circular locus.
  - SVPWM scope: Hexagon with sectors 1 to 6, reference voltage vector $V^*$, and maximum linear modulation circle ($V_{\text{bus}} / \sqrt{3}$).
  - Gimbal 3D projection: Wireframe pitch/roll gimbal orientation render.
- **Rationale**: These are geometric/state diagrams rather than time-series strip charts; direct canvas drawing allows 60 FPS drawing with under 150 lines of JS per widget.

### Decision 3: Use DaisyUI 5 semantic component styling with surface-active highlights
- **Choice**: Utilize DaisyUI 5 cards, tabs (`tab tab-bordered`), joins (`join join-horizontal`), badges, kbd shortcuts, and modals.
- **Constraint Compliance**:
  - Under no circumstance use dot indicators (`.active::after { content: ''; width: 4px; ... }`). Active tabs and buttons must use surface background tint (`bg-base-300`, `border-primary`, `text-primary`).
  - Viewport must be full bleed (`height: calc(100vh - 48px)`), dock anchored to `bottom: 0`.
  - Zero emojis in all labels, tooltips, buttons, logs, and documentation.

## Risks / Trade-offs

- **[High Telemetry Bandwidth on WebSocket]** -> High-rate telemetry (100 Hz+) can saturate browser UI threads.
  *Mitigation*: uPlot uses typed arrays (`Float32Array`) and requestAnimationFrame batching, capping render cycles to 60 FPS while retaining a 2,000-point sliding window.
- **[Multi-Motor State Synchronization]** -> In multi-motor topologies, switching between M0..M3 could lose pending input values.
  *Mitigation*: Maintain an in-memory client state cache for each motor (`clientState[motorIndex]`) and update the UI atomically when switching tabs.
