## ADDED Requirements

### Requirement: MCP Servo Attach Tool
The system SHALL expose `servo_attach` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Successfully attaching a servo pin
- **WHEN** client invokes `tools/call` with name `servo_attach`, `pin` parameter (e.g. 16), and optional `min_us`, `max_us`, `continuous`
- **THEN** system claims the pin under `Driver::Servo`, configures 50 Hz PWM, and returns success with channel details

#### Scenario: Attaching a conflicting pin
- **WHEN** client invokes `tools/call` for `servo_attach` on a pin already claimed by another driver
- **THEN** system returns `isError: true` and an explanatory error message

### Requirement: MCP Servo Write Angle Tool
The system SHALL expose `servo_write_angle` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Setting positional angle
- **WHEN** client invokes `tools/call` with name `servo_write_angle`, `pin` parameter, and `angle` between 0 and 180
- **THEN** system updates the PWM compare value and returns the resulting `pulse_us` and angle

#### Scenario: Out of range angle
- **WHEN** client invokes `tools/call` with an invalid angle
- **THEN** system returns `isError: true` with error description

### Requirement: MCP Servo Write Microseconds Tool
The system SHALL expose `servo_write_us` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Setting direct microsecond pulse
- **WHEN** client invokes `tools/call` with name `servo_write_us`, `pin` parameter, and `pulse_us`
- **THEN** system clamps to channel limits, writes compare register, and returns updated pulse width

### Requirement: MCP Servo Detach Tool
The system SHALL expose `servo_detach` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Detaching active channel
- **WHEN** client invokes `tools/call` with name `servo_detach` and `pin` parameter
- **THEN** system stops PWM output, releases the pin to High-Z in the arbiter, and returns success

### Requirement: MCP Manifest Accuracy
The system SHALL list accurate tool capabilities in `webui/mcp.json`.

#### Scenario: Tool count reflection
- **WHEN** client fetches `/mcp.json`
- **THEN** the manifest reports tool count as 40
