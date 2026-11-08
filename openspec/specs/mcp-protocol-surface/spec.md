# mcp-protocol-surface Specification

## Purpose
Model Context Protocol (MCP) JSON-RPC 2.0 agent endpoint exposing hardware tool registry and safety checks.
## Requirements
### Requirement: JSON-RPC method dispatch
The firmware SHALL implement a minimal Server that accepts POST bodies JSON-RPC 2.0 and responds with one response packet.

#### Scenario: initialize
- **WHEN** `initialize` arrives with client info
- **THEN** server responds `{jsonrpc, result:{protocolVersion, serverInfo, capabilities, instructions}, id}`.

#### Scenario: Unsupported method
- **WHEN** a method other than supported (`initialize`, `tools/list`, `tools/call`, `notifications/initialized`, `ping`) arrives
- **THEN** server responds `-32601 Method not found`.

### Requirement: Tool registry
The system SHALL register a bounded set of hardware tools and dispatch by `tools/call` name.

#### Scenario: List
- **WHEN** `tools/list` is called
- **THEN** server emits an array of `{name, description, inputSchema}` for every available tool.

#### Scenario: Call unknown tool
- **WHEN** `tools/call` invokes an unregistered tool
- **THEN** server responds `isError=true` with text `"Unknown tool: <name>"`.

### Requirement: Tool result envelope
All tool results SHALL be returned inside the MCP text content envelope, and errors SHALL use `isError` plus explanatory text.

#### Scenario: Successful tool
- **WHEN** a registered tool runs within its declared block
- **THEN** payload is wrapped as `{content:[{type:"text",text:"<JSON>"}], isError:false}`.

### Requirement: Safety
The system SHALL refuse pin-claiming tool calls that cannot secure a pin (registry conflict), and `system_reboot_bootsel` shall schedule the reset after 50 ms.

#### Scenario: Pin conflict
- **WHEN** a tool fails to reserve a needed pin
- **THEN** tool returns `isError:true` text mentioning the conflicting pin.

### Requirement: Autonomous composite diagnostic tool exposure
The system SHALL expose `mcp_target_diagnose`, `mcp_inspect_firmware`, and `mcp_sniff_bus` in `tools/list` and dispatch their execution in `tools/call`.

#### Scenario: Tool list discovery
- **WHEN** client issues `tools/list`
- **THEN** response includes `mcp_target_diagnose`, `mcp_inspect_firmware`, and `mcp_sniff_bus` with JSON input schemas

#### Scenario: Dispatch composite tool
- **WHEN** client issues `tools/call` for `mcp_target_diagnose`
- **THEN** server executes the physical and CoreSight diagnostics and returns structured JSON in the content envelope

### Requirement: Universal Chip Catalog Query via MCP
The MCP server SHALL expose `catalog_lookup` in `tools/list` and dispatch it to `agentla_targets::catalog`.

#### Scenario: Lookup target chip by part number
- **WHEN** client invokes `tools/call` with name `catalog_lookup` and query `"STM32F401"`
- **THEN** server returns JSON with target family, Flash/RAM base addresses, size bounds, and unbrick algorithm availability.

### Requirement: Multi-Vendor Target Unbricking via MCP
The MCP server SHALL expose `programmer_unbrick` in `tools/list` and dispatch it to `agentla_targets::engines::rescue`.

#### Scenario: Execute unbrick sequence
- **WHEN** client invokes `tools/call` with name `programmer_unbrick` specifying target family
- **THEN** firmware executes the hardware unlock/mass-erase sequence and returns success confirmation.

### Requirement: Parameterized Flash Programmer Tools
The `programmer_read` and `programmer_flash` MCP tools SHALL accept exact address and byte parameters with strict boundary checks.

#### Scenario: Read target memory range
- **WHEN** client invokes `tools/call` with name `programmer_read` specifying `address` and `length` (up to 256 bytes)
- **THEN** firmware reads target memory via SWD and returns hex-encoded bytes.

#### Scenario: Flash target memory chunk
- **WHEN** client invokes `tools/call` with name `programmer_flash` specifying `address` and `hex_data`
- **THEN** firmware writes the bytes to target flash and returns verification status.

