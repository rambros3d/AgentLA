## ADDED Requirements

### Requirement: Standard MCP Discovery Manifest
The WebUI SHALL serve a valid JSON manifest at `/mcp.json` declaring the probe's MCP server name, version, and transport endpoints.

#### Scenario: Autodiscovery by AI coding agent
- **WHEN** an external MCP client requests `GET /mcp.json`
- **THEN** server returns HTTP 200 with `application/json` containing `name`, `version`, `transports` (HTTP and SSE), and `capabilities`.

### Requirement: Default Application SDK Adoption
The default LittleFS applications (`apps/servo-master`, `apps/can-sniffer`, `apps/terminal`) SHALL utilize `window.AgentLA` for hardware communication rather than manual fetch and WebSocket implementations.

#### Scenario: CAN Sniffer operates via SDK
- **WHEN** user opens `/apps/can-sniffer/`
- **THEN** the app connects and streams CAN traffic using `AgentLA.can.openStream()` with automatic reconnect and termination control.

#### Scenario: Servo Master operates via SDK
- **WHEN** user opens `/apps/servo-master/`
- **THEN** servo attachment and angle updates are executed through `AgentLA.servo.attach()` and `AgentLA.servo.writeAngle()`.
