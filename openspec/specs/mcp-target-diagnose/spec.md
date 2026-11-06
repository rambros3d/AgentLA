# mcp-target-diagnose Specification

## Purpose
TBD - created by archiving change ai-agent-mcp-hardware-tools. Update Purpose after archive.
## Requirements
### Requirement: One-shot target electrical and CoreSight diagnostic
The system SHALL provide an autonomous MCP tool `mcp_target_diagnose` that evaluates target power rails, pin pull-up/drive integrity, CoreSight DPIDR identification, and CPU execution state in a single call.

#### Scenario: Healthy target diagnostic
- **WHEN** `mcp_target_diagnose` is invoked on an operating Cortex-M target
- **THEN** the system returns nominal rail voltage, verified pin integrity, target IDCODE, and status indicating the core is running normally

#### Scenario: Target in HardFault loop
- **WHEN** `mcp_target_diagnose` is invoked on a crashed target
- **THEN** the system reads `DHCSR`, `CFSR`, `HFSR`, and `BFAR`, identifies the fault type, captures the faulting Program Counter, and returns root-cause analysis

#### Scenario: Unpowered target detection
- **WHEN** `mcp_target_diagnose` is invoked on an unpowered target (rail < 0.5V)
- **THEN** the system returns `rail_status: "unpowered"` and pin state report without entering infinite connection retry loops

