## 1. uPlot Library Integration

- [x] 1.1 Download and bundle `uplot.min.js` and `uplot.min.css` into `webui/`.
- [x] 1.2 Include uPlot assets in `webui/barefoc.html` header and verify offline instantiation.

## 2. DaisyUI 5 Layout & Navigation Shell

- [x] 2.1 Reconstruct `webui/barefoc.html` container layout using DaisyUI 5 with full-bleed viewport (`height: calc(100vh - 48px)`) and bottom dock anchoring.
- [x] 2.2 Implement header topology preset selector (4x 1-DOF, 2x 2-DOF, 1x 1-DOF + 1x 2-DOF, 2x 1-DOF + 1x 2-DOF) writing to MultiBus register `0x78`.
- [x] 2.3 Implement motor tab bar (M0..M3) with surface-active styling (`bg-base-300 border-primary text-primary`), strictly ensuring zero CSS dot indicators.

## 3. FOC Vector Scope & Space Vector PWM Visualizers

- [x] 3.1 Implement Clarke/Park polar locus canvas rendering $I_\alpha, I_\beta$ circular trajectory and rotating $I_d, I_q$ vector circle.
- [x] 3.2 Implement SVPWM space vector hexagon canvas displaying sectors 1..6, reference voltage vector $V^*$, and maximum linear modulation circle.
- [x] 3.3 Add 3-phase bar meters displaying PWM duties ($T_a, T_b, T_c$) and instantaneous phase currents ($I_a, I_b, I_c$).

## 4. Interactive Cascaded Control Loop Diagram

- [x] 4.1 Implement interactive SVG/HTML signal flow schematic: Target -> Outer Joint PID -> P-Angle -> PID-Velocity -> Current DQ -> SVPWM -> Inverter -> Motor -> Sensor.
- [x] 4.2 Add inline parameter tuning pills on block diagram nodes with live MultiBus register synchronization.

## 5. uPlot Real-Time Telemetry Oscilloscopes

- [x] 5.1 Implement dual/quad-trace strip chart for setpoint trajectory vs feedback angle and velocity with synchronized cursor readouts.
- [x] 5.2 Implement 3-phase and DQ current strip chart ($I_a, I_b, I_c, I_d, I_q$) with rolling buffer and pause/CSV export.

## 6. Kinematics Workbench & Transports

- [x] 6.1 Implement 2-DOF differential gimbal 3D-projected pitch/roll orientation visualizer.
- [x] 6.2 Implement interactive LUT transmission spline editor with drag handles and auto-increment burst I2C writing on register `0xA4`.
- [x] 6.3 Implement automated calibration wizard with real-time feedback (`0xB0..0xB4`).
- [x] 6.4 Integrate SimpleFOC ASCII Commander terminal with command history, syntax styling, and quick-action shortcuts.
