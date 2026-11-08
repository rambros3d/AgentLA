## ADDED Requirements

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
