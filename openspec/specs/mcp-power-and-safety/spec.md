# mcp-power-and-safety Specification

## Purpose
TBD - created by archiving change expand-agentla-mcp-tools. Update Purpose after archive.
## Requirements
### Requirement: Target Power Control Tool
The MCP server SHALL provide a `power_control` tool to switch the target power rail state and set the output voltage level.

#### Scenario: Enabling target power rail
- **WHEN** client invokes MCP tool `power_control` with `enabled: true` and optional `voltage_mv`
- **THEN** server sets the power switch state, regulates target voltage, and returns the measured voltage and current telemetry

#### Scenario: Disabling target power rail
- **WHEN** client invokes MCP tool `power_control` with `enabled: false`
- **THEN** server immediately opens the target power rail switch

### Requirement: Target Power Cycle Tool
The MCP server SHALL provide a `power_cycle` tool to reboot target microcontrollers by pulsing the power rail off and back on.

#### Scenario: Executing power cycle
- **WHEN** client invokes MCP tool `power_cycle` with `delay_ms: 100`
- **THEN** server opens the power switch for the specified duration, re-enables it, and returns confirmation

### Requirement: Atomic Emergency Safe State Tool
The MCP server SHALL provide an `emergency_safe_state` tool that transitions all GPIO/PWM pins to High-Z and cuts target power in a single operation.

#### Scenario: Triggering emergency safe state
- **WHEN** client invokes MCP tool `emergency_safe_state`
- **THEN** server immediately releases all pin ownerships to High-Z, opens the target power rail switch, and returns confirmation

