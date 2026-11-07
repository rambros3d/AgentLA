## MODIFIED Requirements

### Requirement: Generic algorithm as primary flash implementation
The system SHALL support a 3-tier flash resolution cascade: executing direct register engines or ROM-embedded RAM loaders (Tier 1) for catalog targets, executing LittleFS-resident FLM blobs (Tier 2) under `/system/algo/<name>.bin`, and streaming host-supplied FLM payloads (Tier 3) into target RAM.

#### Scenario: Direct engine execution for catalog target
- **WHEN** a recognized catalog target is programmed and no override is requested
- **THEN** the system executes the assigned direct register engine or ROM-embedded RAM loader (Tier 1) without requiring LittleFS access.

#### Scenario: LittleFS FLM execution for extended targets
- **WHEN** an uncataloged target has a matching `.bin` algorithm installed in LittleFS `/system/algo/`
- **THEN** the system reads the 44-byte `AlgoBlobHeader`, streams the executable into target SRAM, and executes `ProgramPage` (Tier 2).

#### Scenario: Host FLM dynamic streaming
- **WHEN** a target is neither in Tier 1 nor Tier 2, but the host/MCP client streams an FLM blob payload
- **THEN** the system loads the provided RAM algorithm and completes programming (Tier 3).
