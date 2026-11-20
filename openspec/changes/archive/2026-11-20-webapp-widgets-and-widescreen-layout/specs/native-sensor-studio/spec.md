## ADDED Requirements

### Requirement: 3D IMU Orientation Viewer via OGL
Sensor Studio SHALL integrate `ogl.min.js` to provide a real-time 3D attitude viewer for IMU sensors (MPU6050, BNO055, ICM-42688) rotating a 3D board/cube based on incoming quaternion telemetry $[w, x, y, z]$.

#### Scenario: Streaming IMU quaternions
- **WHEN** client streams quaternion telemetry from a connected IMU
- **THEN** OGL 3D canvas updates mesh orientation in real time at 60 FPS without DOM reflow

### Requirement: Sensor Studio Multi-Column Widescreen Layout
Sensor Studio SHALL organize bus scanning, sensor profile registries, and live 3D/strip-chart visualizers into a responsive 12-column widescreen grid.

#### Scenario: Widescreen sensor dashboard
- **WHEN** user views Sensor Studio on a wide screen
- **THEN** bus controls and register tables occupy 4 columns, and 3D IMU attitude viewer and uPlot charts occupy 8 columns side-by-side
