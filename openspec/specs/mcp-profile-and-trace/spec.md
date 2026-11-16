# mcp-profile-and-trace Specification

## Purpose
TBD - created by archiving change expand-agentla-mcp-tools. Update Purpose after archive.
## Requirements
### Requirement: Hardware Profile Management Tools
The MCP server SHALL provide `profile_list` and `profile_apply` tools to enumerate and activate non-volatile hardware profile slots.

#### Scenario: Listing hardware profile slots
- **WHEN** client invokes MCP tool `profile_list`
- **THEN** server returns a JSON array of profile slots with active slot indicator and slot descriptions

#### Scenario: Applying hardware profile slot
- **WHEN** client invokes MCP tool `profile_apply` with `slot: 2`
- **THEN** server applies the hardware pin configuration stored in slot 2 and returns success status

### Requirement: Serial Wire Output Trace Readout Tool
The MCP server SHALL provide a `swo_read` tool to drain captured ITM/SWO printf trace bytes from Cortex-M debug sessions.

#### Scenario: Reading captured SWO trace
- **WHEN** client invokes MCP tool `swo_read`
- **THEN** server returns the buffered SWO trace string or byte stream and optionally clears the backlog buffer

