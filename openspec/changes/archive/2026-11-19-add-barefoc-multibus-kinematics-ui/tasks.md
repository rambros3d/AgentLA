## 1. Multi-Motor Selector UI

- [x] 1.1 Add multi-motor tabs to `webui/barefoc.html` with active surface styling (no CSS dot indicators).
- [x] 1.2 Implement `setMotorIndex(m)` dispatching write to `0x70` and refreshing active telemetry in `MultiBusAdapter`.

## 2. Kinematics Panel Implementation

- [x] 2.1 Add Kinematics tab with parameter sliders for Gear Ratio, Backlash Delta, Omega Thresh, and Torsional Stiffness.
- [x] 2.2 Add 2D Pitch/Roll angle indicators and setpoint target inputs.
- [x] 2.3 Add outer PID sliders for Alpha and Beta axes.
- [x] 2.4 Add 2x2 differential coupling matrix editor.

## 3. LUT Curve Streaming Tool

- [x] 3.1 Implement HTML5 canvas curve visualizer with drag/upload support.
- [x] 3.2 Implement burst I2C write streaming across registers 0xA0–0xA4.
- [x] 3.3 Display monotonicity status badge based on register 0xA5.

## 4. Calibration & Flash Persistence UI

- [x] 4.1 Add calibration command buttons (Stall Discovery, Interactive Step, Abort).
- [x] 4.2 Add Save to Flash and Load from Flash buttons with CRC integrity badge.
- [x] 4.3 Validate strict zero-emoji compliance and verify UI behavior.
