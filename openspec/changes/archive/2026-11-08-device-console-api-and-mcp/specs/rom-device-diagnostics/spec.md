## ADDED Requirements

### Requirement: Multi-View Tab Navigation and Direct Subpath Routing
The In-ROM diagnostics console (`device.html`) SHALL provide a top-level tab switcher supporting three dedicated view panels: `[ HARDWARE ]` (`#hardware`), `[ REST API ]` (`#api`), and `[ MCP AGENT ]` (`#mcp`). The firmware HTTP router SHALL register route aliases for `/device/api`, `/device/api/`, `/device/mcp`, and `/device/mcp/` serving the same compiled Flash ROM template, and the client JavaScript SHALL synchronize the active tab automatically with the URL path or hash without triggering a full page reload.

#### Scenario: Navigating directly to /device/mcp
- **WHEN** an operator or AI agent navigates to `http://10.10.10.10/device/mcp`
- **THEN** the server returns HTTP 200 OK serving `device.html` and client JavaScript automatically activates the `[ MCP AGENT ]` tab panel.

#### Scenario: Switching tabs in the browser
- **WHEN** the user clicks the `[ REST API ]` tab header while on the Hardware view
- **THEN** the hardware panel hides, the REST API catalog displays immediately in 0 ms, and the browser URL hash updates to `#api`.

### Requirement: Interactive REST API Catalog and Endpoint Runner
The diagnostics console SHALL provide a categorized reference table of all native firmware endpoints (System Telemetry, Pin Arbiter, Peripheral Transceivers, Servo/PWM, Firmware Management, and WebSockets). Each entry SHALL display the HTTP method badge, route path, description, a copyable `curl` snippet, and a live "Try Endpoint" runner allowing developers to test endpoints with live JSON response feedback directly in the browser.

#### Scenario: Copying a curl command for telemetry
- **WHEN** the user views `/api/v1/system/telemetry` in the REST API tab and clicks "Copy curl"
- **THEN** a complete `curl -s http://<host>/api/v1/system/telemetry` command is copied to the clipboard with an inline confirmation indicator.

#### Scenario: Executing a live API query from the console
- **WHEN** the user clicks "Try Endpoint" on `/api/board/manifest`
- **THEN** the console issues an in-browser `fetch()` call and displays the formatted JSON response and HTTP status code in an adjacent response viewer.

### Requirement: MCP Agent Bridge Integration and Schema Reference
The diagnostics console SHALL provide a dedicated MCP Agent view documenting the probe's Model Context Protocol server. The view SHALL state the HTTP endpoint (`/mcp`) and SSE stream (`/mcp/sse`), provide a 1-click copyable `mcpServers.agentla` configuration snippet for Google Antigravity, Anthropic Claude Desktop, and Cursor/OpenCode, and list all native on-probe tools with their JSON-RPC method names, input schemas, and functional descriptions.

#### Scenario: Copying AI agent MCP configuration
- **WHEN** the user clicks "Copy Config" in the MCP Agent tab
- **THEN** a validated JSON snippet containing `"agentla": { "url": "http://<host>/mcp" }` is copied to the system clipboard.

#### Scenario: Inspecting on-probe MCP tools
- **WHEN** viewing the MCP Agent panel
- **THEN** the console enumerates `mcp_target_diagnose`, `mcp_inspect_firmware`, `mcp_sniff_bus`, `mcp_pins_claim`, and `mcp_telemetry_get` alongside their respective argument parameters.
