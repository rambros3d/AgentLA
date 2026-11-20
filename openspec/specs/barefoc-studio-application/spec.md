# barefoc-studio-application Specification

## Purpose
Legacy USER_FS application specification. The app was promoted to a built-in SYSTEM_FS application by the native-barefoc-system-app change; current behavior is tracked by native-barefoc-system-app and the motor-* parity capabilities.
## Requirements
### Requirement: Catalog Registration and Installation
The system SHALL register the BareFOC Studio application in `apps/catalog.json` and support installation via the AgentLA App Store.

#### Scenario: Catalog entry presence
- **WHEN** the user browses the App Store
- **THEN** BareFOC Studio is listed with icon, description, tags ("Robotics", "Motor", "FOC", "ODrive"), and download link `/packages/barefoc.tar`

#### Scenario: LittleFS installation execution
- **WHEN** the user triggers installation of BareFOC Studio
- **THEN** the system unpacks the tarball into LittleFS `/apps/barefoc/` and registers the app on the dock and launcher

### Requirement: Unified Multi-Protocol Communication
The application SHALL support Fibre RPC, SimpleFOC Commander, and MultiBus protocol lanes through a unified client abstraction.

#### Scenario: Fibre RPC communication
- **WHEN** the user selects Fibre protocol and connects
- **THEN** the client reads endpoint 0 schema, extracts motor/controller endpoints, and exchanges binary frames

#### Scenario: SimpleFOC Commander communication
- **WHEN** the user selects Commander protocol
- **THEN** the client transmits formatted ASCII strings over the AgentLA UART WebSocket and parses telemetry responses

#### Scenario: MultiBus register table communication
- **WHEN** the user selects MultiBus protocol
- **THEN** the client reads and writes register offsets corresponding to target, control mode, and PID tables

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

### Requirement: Motor and Sensor Setup Wizard
The application SHALL provide a guided wizard for motor parameters, sensor selection, and automated electrical zero angle calibration.

#### Scenario: Running electrical zero alignment
- **WHEN** the user triggers the alignment sequence in the wizard
- **THEN** the target executes `init_foc()`, searches for rotor electrical zero, and reports discovered zero angle and direction

### Requirement: Hardware Safety and Emergency State Integration
The application SHALL integrate with `agentla-bridge.js` to monitor target power and provide emergency safe-state triggers.

#### Scenario: Emergency safe-state activation
- **WHEN** the user clicks the Emergency Cut-Off button or presses Shift+Esc
- **THEN** `AgentLA.emergencySafeState()` is invoked, immediately placing all PWM pins in High-Z and opening the target power switch

#### Scenario: Standalone simulation fallback
- **WHEN** the application is loaded with `?demo=1` or offline
- **THEN** an internal virtual motor simulator generates synthetic telemetry curves and step responses

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

### Requirement: Board and Joint Identity Display
The BareFOC Studio application SHALL display silicon hardware UID, board name, board node ID, and per-joint addresses and names.

#### Scenario: Displaying board identity
- **WHEN** the application connects to a BareFOC target
- **THEN** it queries `REG_HARDWARE_UID`, `REG_BOARD_NAME`, and `REG_BOARD_NODE_ID` and displays them in the status strip.

#### Scenario: Editing joint identity
- **WHEN** the user selects a motor tab (M0..M3) and updates the joint address or name
- **THEN** the application writes the updated values to `REG_JOINTx_ADDR` and `REG_JOINTx_NAME` over MultiBus.

### Requirement: Enhanced Calibration Wizard and Safe Disarm Status
The BareFOC Studio application SHALL display motor calibration state, electrical zero angle readout, 1-click ALIGN auto-commit, and mechanical zeroing controls.

#### Scenario: Executing ALIGN from Studio
- **WHEN** the user clicks `Run Electrical ALIGN`
- **THEN** the application writes `0x01` to MultiBus register `0xB0`, updates the state indicator during alignment, and verifies auto-commit to Flash.

