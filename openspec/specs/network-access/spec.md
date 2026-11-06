# network-access Specification

## Purpose
Enables command-capable network interface over USB-RNDIS, MCP discovery, REST API endpoints, SSE streams, and dual-core network isolation.

## Requirements

### Requirement: Probe-side network access
The system SHALL serve an actual command-capable network interface from the RP2350 over USB-RNDIS, including structured command request and result channels, command handle support, method discovery via `/mcp`, dedicated REST API routing under `/api/v1`, and multi-app LittleFS file streaming under `/apps`.

#### Scenario: Agent connects
- **WHEN** LLM agent connects to probe IP (pico.lab/agentla.lab)
- **THEN** `/mcp` SHALL support discovery of allowed command tools and execute permitted commands.

#### Scenario: Browser calls command
- **WHEN** a WebUI sends a valid JSON-RPC command request
- **THEN** firmware SHALL route it to the existing command dispatcher and return a structured response.

#### Scenario: Client accesses REST API
- **WHEN** an external tool or web application sends an HTTP request to `/api/v1/*`
- **THEN** firmware SHALL dispatch the request directly to REST handlers and return JSON with standard HTTP status codes.

### Requirement: Web GUI
The system SHALL serve a WinBox-style desktop web UI over HTTP from flash.

#### Scenario: Browser loads
- **WHEN** user opens http://agentla.lab
- **THEN** UI renders with Flasher/Analyzer/Debugger/GPIO/Bus windows.

### Requirement: SSE streams
The system SHALL provide SSE endpoints for live data capture and progress.

#### Scenario: Capture stream
- **WHEN** client subscribes /sse/capture
- **THEN** RLE-compressed samples pushed over HTTP.

### Requirement: Dual-core operation
The system SHALL keep capture/decode on Core 1 and network/UI on Core 0.

#### Scenario: Core split
- **WHEN** system boots
- **THEN** Core 0 hosts network stack; Core 1 runs capture loop.

### Requirement: Dual-core capability remains behaviorally consistent
The system SHALL document whether network command execution and capture may share a core, and SHALL avoid claiming passive asynchronous operation without a tested core-split implementation.

#### Scenario: Implementation status
- **WHEN** network features are verified
- **THEN** the definition of dual-core operation SHALL reflect an actual launched Core 1 or a revised single-core contract.
