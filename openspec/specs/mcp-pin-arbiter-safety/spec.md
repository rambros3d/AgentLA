# mcp-pin-arbiter-safety Specification

## Purpose
TBD - created by archiving change unified-mcp-and-javascript-bridge-sdk. Update Purpose after archive.
## Requirements
### Requirement: Pin Allocation Matrix Query via MCP
The MCP server SHALL expose `pins_status` in `tools/list` returning the driver ownership and electrical state of all 30 GPIO pins (GP0-GP29).

#### Scenario: Query pin crossbar state
- **WHEN** client invokes `tools/call` with name `pins_status`
- **THEN** server returns structured JSON containing the list of all pins, their active driver claims (`None`, `SWD`, `UART`, `I2C`, `SPI`, `CAN`, `PWM`), and overcurrent alarm state.

### Requirement: Atomic Pin Claim and Release via MCP
The MCP server SHALL expose `pins_claim` and `pins_release` to enforce atomic ownership before agents drive hardware signals.

#### Scenario: Agent acquires pin ownership
- **WHEN** client invokes `tools/call` with name `pins_claim` for an unclaimed pin and valid driver
- **THEN** firmware reserves the pin under `agentla-pin-arbiter` and returns success.

#### Scenario: Agent handles pin contention
- **WHEN** client invokes `tools/call` with name `pins_claim` on a pin already reserved by another driver
- **THEN** firmware rejects the request with `isError: true` specifying the conflicting driver and pin index.

#### Scenario: Agent releases pins to High-Z
- **WHEN** client invokes `tools/call` with name `pins_release`
- **THEN** firmware transitions specified pins back to High-Z input and clears driver claims.

### Requirement: Structured JSON System Status
The MCP `status` tool SHALL return compact, structured JSON telemetry rather than unstructured key-value text.

#### Scenario: Client queries system status
- **WHEN** client invokes `tools/call` with name `status`
- **THEN** server returns JSON with `clock_mhz`, `core_temp_c`, `v_target_mv`, `i_target_ma`, `active_profile`, and `status`.

