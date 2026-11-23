# Spec Delta

## MODIFIED Requirements

### Requirement: Authoritative Mcp Catalog Counts
The MCP tool, resource, and prompt counts SHALL be derived from the actual `ALL_TOOLS`, `DEFAULT_RESOURCES`, and `DEFAULT_PROMPTS` array lengths, accommodating the expanded control-plane toolset, and every manifest, health response, and test assertion SHALL use those authoritative values.

#### Scenario: Manifest matches catalog
- **WHEN** the system serves `/mcp.json`, `/mcp/health`, or any test asserts the MCP catalog size
- **THEN** the reported tool, resource, and prompt counts equal the compiled array lengths (matching `ALL_TOOLS.len()` = 78 tools)

#### Scenario: Catalog includes BareFOC entries
- **WHEN** the catalog is inspected after this change
- **THEN** it includes the eight `barefoc_*` tools, the `agentla://skills/barefoc` resource, and the `barefoc_debug` prompt

#### Scenario: Verify expanded tool catalog consistency
- **WHEN** client queries `tools/list` or `/mcp/health`
- **THEN** server returns the authoritative tool count matching `ALL_TOOLS.len()` and matches `webui/mcp.json`.
