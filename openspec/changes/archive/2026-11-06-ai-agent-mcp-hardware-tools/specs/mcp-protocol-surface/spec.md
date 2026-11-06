## ADDED Requirements

### Requirement: Autonomous composite diagnostic tool exposure
The system SHALL expose `mcp_target_diagnose`, `mcp_inspect_firmware`, and `mcp_sniff_bus` in `tools/list` and dispatch their execution in `tools/call`.

#### Scenario: Tool list discovery
- **WHEN** client issues `tools/list`
- **THEN** response includes `mcp_target_diagnose`, `mcp_inspect_firmware`, and `mcp_sniff_bus` with JSON input schemas

#### Scenario: Dispatch composite tool
- **WHEN** client issues `tools/call` for `mcp_target_diagnose`
- **THEN** server executes the physical and CoreSight diagnostics and returns structured JSON in the content envelope
