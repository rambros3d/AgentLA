## MODIFIED Requirements

### Requirement: Interactive Motion Control and Cascaded Tuning
The application SHALL provide an interactive visual block diagram of the cascaded control loops (Outer Joint PID -> P-Angle -> PID-Velocity -> Current DQ -> SVPWM -> Inverter -> Motor -> Sensor) with real-time controls for all 5 motion modes, 3 torque modes, and inline tuning pills for PID/LPF parameters.

#### Scenario: Changing motion control mode
- **WHEN** the user selects a motion control mode (torque, velocity, angle, velocity_openloop, angle_openloop)
- **THEN** the application dispatches the mode update command to the connected target and updates active signal paths on the block diagram

#### Scenario: Adjusting cascaded PID parameters
- **WHEN** the user adjusts a PID gain or limit on the diagram or control panel
- **THEN** the new parameter is transmitted immediately to the device and reflected in telemetry

### Requirement: High-Performance Canvas Oscilloscope
The application SHALL render high-performance multi-channel strip charts using `uPlot` plotting setpoint trajectory, measured mechanical angle, angular velocity, and phase/DQ currents at up to 60 FPS.

#### Scenario: Live waveform rendering
- **WHEN** the motor is operating in closed-loop mode
- **THEN** `uPlot` plots setpoint trajectories and actual feedback on synchronized multi-trace charts with interactive cursor readouts

#### Scenario: Freezing and exporting telemetry
- **WHEN** the user clicks "Freeze" and "Export CSV"
- **THEN** the plot halts rolling updates and downloads recorded buffer points as a CSV file

## ADDED Requirements

### Requirement: FOC Vector Scope and SVPWM Visualizer
The application SHALL render dedicated polar and geometric visualizers for field-oriented control states, including Clarke transform $I_\alpha, I_\beta$ locus, Park transform $I_d, I_q$ vector circle, and Space Vector PWM (SVPWM) sector hexagon with reference voltage trajectory.

#### Scenario: Real-time polar locus display
- **WHEN** high-frequency current telemetry is streamed from the motor controller
- **THEN** the FOC vector scope draws the $I_\alpha, I_\beta$ trajectory and $I_d, I_q$ vector relative to the rotor electrical angle $\theta_e$

#### Scenario: SVPWM sector and modulation inspection
- **WHEN** voltage commands are computed by the FOC loop
- **THEN** the SVPWM hexagon highlights the active switching sector (1..6) and draws the voltage vector $V^*$ against the maximum linear modulation circle

### Requirement: Kinematics and Multi-Motor Topology Workbench
The application SHALL provide a multi-motor kinematics workbench supporting 4-motor topology presets (4x 1-DOF, 2x 2-DOF, 1x 1-DOF + 1x 2-DOF, 2x 1-DOF + 1x 2-DOF), motor tab switching (M0..M3), 2-DOF differential gimbal preview, non-linear transmission spline editor with I2C burst writing, and automated calibration trigger controls.

#### Scenario: Dynamic topology switching
- **WHEN** the user selects a topology preset from the header pill
- **THEN** the application writes the corresponding preset value to MultiBus register `0x78` (`TopologyMode`) and reconfigures the UI controls to match the active topology

#### Scenario: Non-linear LUT spline streaming
- **WHEN** the user modifies points on the LUT curve editor and clicks "Flash Write"
- **THEN** the application transmits the lookup table via MultiBus auto-increment burst write on register `0xA4` and reports status

### Requirement: DaisyUI 5 Component Shell and Zero-Dot Surface Styling
The application SHALL conform strictly to DaisyUI 5 semantic component standards and AgentLA UI constraints: active states MUST be conveyed exclusively through surface styling (background tints or border highlights), never with floating CSS dot indicators; the application viewport MUST remain full-bleed (`height: calc(100vh - 48px)`); the dock MUST remain anchored to `bottom: 0`; and all labels, logs, and elements MUST be completely free of emojis.

#### Scenario: Surface-styled active tab indication
- **WHEN** the user clicks a motor tab (M0..M3) or navigation view
- **THEN** the active tab receives `bg-base-300 border-primary text-primary` surface styling without any `.active::after` pseudo-element dot indicators
