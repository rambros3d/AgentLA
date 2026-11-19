# domain-crate-mcp Specification

## Purpose
TBD - created by archiving change modularize-mcp-crate. Update Purpose after archive.
## Requirements
### Requirement: Standalone MCP Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-mcp` configured as a `#![no_std]` library with optional `defmt` and `std` features.

#### Scenario: Compiling and testing on host targets
- **WHEN** building `agentla-mcp` on an x86_64 host with `cargo test --package agentla-mcp`
- **THEN** all JSON-RPC 2.0 schema and tool dispatch unit tests compile and pass.

### Requirement: JSON-RPC 2.0 Protocol Framing
The `agentla-mcp` crate SHALL parse incoming JSON-RPC 2.0 requests and serialize compliant responses including `id`, `result`, and `error` objects without heap allocation.

#### Scenario: Processing tools/list request
- **WHEN** client sends a JSON-RPC request for method `tools/list`
- **THEN** the server returns the static schemas for `inspect_firmware`, `sniff_bus`, and `target_diagnose`.

#### Scenario: Handling invalid JSON-RPC method
- **WHEN** client sends an unrecognized method name
- **THEN** the server returns error code -32601 (`Method not found`).

### Requirement: Authoritative Mcp Catalog Counts
The MCP tool, resource, and prompt counts SHALL be derived from the actual `ALL_TOOLS`, `DEFAULT_RESOURCES`, and `DEFAULT_PROMPTS` array lengths, and every manifest, health response, and test assertion SHALL use those authoritative values.

#### Scenario: Manifest matches catalog
- **WHEN** the system serves `/mcp.json`, `/mcp/health`, or any test asserts the MCP catalog size
- **THEN** the reported tool, resource, and prompt counts equal the compiled array lengths (no stale 14/40/45/49/2 values)

#### Scenario: Catalog includes BareFOC entries
- **WHEN** the catalog is inspected after this change
- **THEN** it includes the eight `barefoc_*` tools, the `agentla://skills/barefoc` resource, and the `barefoc_debug` prompt

