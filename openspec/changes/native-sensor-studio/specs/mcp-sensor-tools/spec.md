## ADDED Requirements

### Requirement: MCP Sensor Scan Tool
The system SHALL expose `sensor_scan` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Scanning I2C bus
- **WHEN** client invokes `tools/call` with name `sensor_scan` and bus type `i2c`
- **THEN** system returns an array of responding addresses with matched sensor profile names

### Requirement: MCP Sensor Read Register Tool
The system SHALL expose `sensor_read_reg` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Reading register bytes
- **WHEN** client invokes `tools/call` with name `sensor_read_reg`, device `addr`, starting `reg`, and `len`
- **THEN** system executes repeated-start read and returns the byte array in hex

### Requirement: MCP Sensor Write Register Tool
The system SHALL expose `sensor_write_reg` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Writing register byte
- **WHEN** client invokes `tools/call` with name `sensor_write_reg`, device `addr`, target `reg`, and `data` bytes
- **THEN** system writes the payload to the device and returns success

### Requirement: MCP Sensor Poll Tool
The system SHALL expose `sensor_poll` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Polling active telemetry
- **WHEN** client invokes `tools/call` with name `sensor_poll`, device `addr`, and `registers` array
- **THEN** system batch reads the requested registers and returns scaled numeric values with units
