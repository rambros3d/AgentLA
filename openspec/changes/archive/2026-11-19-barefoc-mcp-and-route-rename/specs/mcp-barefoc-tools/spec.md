## ADDED Requirements

### Requirement: BareFoc Tool Catalog
The MCP server SHALL advertise a set of BareFOC tools in `tools/list` that wraps the existing hardware bridges for agentic debugging of an external FOC target: `barefoc_status`, `barefoc_target`, `barefoc_read`, `barefoc_write`, `barefoc_command`, `barefoc_mode`, `barefoc_calibrate`, and `barefoc_emergency`.

#### Scenario: Tools are advertised
- **WHEN** an agent calls `tools/list`
- **THEN** the response contains all eight barefoc tool names with JSON input schemas and the catalog count equals the authoritative `ALL_TOOLS` length

#### Scenario: No FOC loop on the probe
- **WHEN** an agent calls a BareFOC tool
- **THEN** the firmware bridges to the external target rather than running a field-oriented control loop locally

### Requirement: BareFoc Status And Target Selection
`barefoc_status` SHALL return fresh target rail telemetry, active profile, and safe-state status. `barefoc_target` SHALL set or query the selected external target node/address used as the default by other tools.

#### Scenario: Reading status
- **WHEN** agent calls `barefoc_status`
- **THEN** the tool samples live telemetry (voltage, current, power, temperature, overcurrent trip) and reports the active profile and power state

#### Scenario: Selecting a target
- **WHEN** agent calls `barefoc_target` with a node or I2C address
- **THEN** the selection is recorded for subsequent tool calls in the session and returned in the response

### Requirement: BareFoc Register Access
`barefoc_read` and `barefoc_write` SHALL access the external target's MultiBus registers over I2C (default address 0x40 + node) or gate-driver registers over SPI, honoring Pin Arbiter ownership.

#### Scenario: Reading a register
- **WHEN** agent calls `barefoc_read` with a node/address and register offset
- **THEN** the tool performs an I2C or SPI read using the active bridge and returns the raw bytes and decoded value

#### Scenario: Writing a register
- **WHEN** agent calls `barefoc_write` with a register offset and bounded payload
- **THEN** the tool writes the bytes over the selected bus and returns an acknowledgment or a NACK error

### Requirement: BareFoc Command And Mode Control
`barefoc_command` SHALL send a SimpleFOC Commander ASCII command over UART and return the response. `barefoc_mode` SHALL command motion mode, torque sub-mode, and enable state over the active CAN/UART transport.

#### Scenario: Sending a commander command
- **WHEN** agent calls `barefoc_command` with a command string such as `M1 V 10`
- **THEN** the tool transmits it over UART and returns the device response with byte counts

#### Scenario: Changing motion mode
- **WHEN** agent calls `barefoc_mode` with a mode such as velocity or angle and an enable flag
- **THEN** the tool emits the corresponding CAN or Commander command and returns the commanded state

### Requirement: BareFoc Calibration And Emergency Safe State
`barefoc_calibrate` SHALL trigger motor or encoder calibration on the external target and report status. `barefoc_emergency` SHALL perform a one-call safe-state: disable the inverter, zero the target, release pins to High-Z, and cut target power.

#### Scenario: Running calibration
- **WHEN** agent calls `barefoc_calibrate` with kind motor or encoder
- **THEN** the tool invokes the target's calibration routine over the active transport and reports in-progress, success, or failure

#### Scenario: Emergency safe state
- **WHEN** agent calls `barefoc_emergency`
- **THEN** the tool disables the inverter, zeroes the setpoint, quiesces claimed pins, cuts target power, and returns confirmation

### Requirement: Bounded No-Std Tool Output
Every BareFOC MCP tool SHALL run in the no_std/zero-heap firmware, accept only bounded arguments, and write bounded output into the tool result buffer.

#### Scenario: Bounded response
- **WHEN** a tool produces a large read response
- **THEN** output is truncated or bounded to the tool result capacity and the MCP response remains valid JSON with `isError` reflecting success or failure
