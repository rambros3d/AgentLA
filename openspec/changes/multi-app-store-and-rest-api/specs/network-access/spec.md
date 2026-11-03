## MODIFIED Requirements

### Requirement: Probe-side network access
The system SHALL serve an actual command-capable network interface from the RP2350 over USB-RNDIS, including structured command request and result channels, command handle support, method discovery via `/mcp`, dedicated REST API routing under `/api/v1`, and multi-app LittleFS file streaming under `/apps`.

#### Scenario: Agent connects
- **WHEN** LLM agent connects to probe IP (pico.lab/agentla.lab)
- **THEN** `/mcp` SHALL support discovery of allowed command tools and execute permitted commands

#### Scenario: Browser calls command
- **WHEN** a WebUI sends a valid JSON-RPC command request
- **THEN** firmware SHALL route it to the existing command dispatcher and return a structured response

#### Scenario: Client accesses REST API
- **WHEN** an external tool or web application sends an HTTP request to `/api/v1/*`
- **THEN** firmware SHALL dispatch the request directly to REST handlers and return JSON with standard HTTP status codes
