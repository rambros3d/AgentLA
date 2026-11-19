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
The application SHALL provide real-time controls for all 5 motion modes, 3 torque modes, and cascaded PID/LPF parameters.

#### Scenario: Changing motion control mode
- **WHEN** the user selects a motion control mode (torque, velocity, angle, velocity_openloop, angle_openloop)
- **THEN** the application dispatches the mode update command to the connected target

#### Scenario: Adjusting cascaded PID parameters
- **WHEN** the user adjusts the Velocity P slider
- **THEN** the new proportional gain is transmitted immediately to the device and reflected in telemetry

### Requirement: High-Performance Canvas Oscilloscope
The application SHALL render a dual-trace real-time oscilloscope plotting target setpoint versus actual feedback at up to 60 FPS.

#### Scenario: Live waveform rendering
- **WHEN** the motor is operating in closed-loop mode
- **THEN** Trace A plots setpoint trajectory and Trace B plots actual feedback on an HTML5 canvas ring buffer

#### Scenario: Freezing and exporting telemetry
- **WHEN** the user clicks "Freeze" and "Export CSV"
- **THEN** the canvas plot halts rolling updates and downloads recorded buffer points as a CSV file

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

