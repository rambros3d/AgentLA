# network-access Specification

## Purpose
TBD - created by archiving change webgui-mcp-agent. Update Purpose after archive.
## Requirements
### Requirement: Probe-side network access
The system SHALL serve MCP-style JSON-RPC endpoints over USB network from RP2350 via RNDIS/DHCP/DNS.

#### Scenario: Agent connects
- **WHEN** LLM agent connects to probe IP (pico.lab/agentla.lab)
- **THEN** /mcp responds with tool list

### Requirement: Web GUI
The system SHALL serve a WinBox-style desktop web UI over HTTP from flash.

#### Scenario: Browser loads
- **WHEN** user opens http://agentla.lab
- **THEN** UI renders with Flasher/Analyzer/Debugger/GPIO/Bus windows

### Requirement: SSE streams
The system SHALL provide SSE endpoints for live data capture and progress.

#### Scenario: Capture stream
- **WHEN** client subscribes /sse/capture
- **THEN** RLE-compressed samples pushed over HTTP

### Requirement: Dual-core operation
The system SHALL keep capture/decode on Core 1 and network/UI on Core 0.

#### Scenario: Core split
- **WHEN** system boots
- **THEN** Core 0 hosts network stack; Core 1 runs capture loop

