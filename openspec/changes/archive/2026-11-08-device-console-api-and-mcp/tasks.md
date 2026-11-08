## 1. Picoserve Router Subpath Aliases

- [x] 1.1 Register `/device/api` and `/device/api/` route aliases in `firmware-rs/src/web.rs`
- [x] 1.2 Register `/device/mcp` and `/device/mcp/` route aliases in `firmware-rs/src/web.rs`

## 2. Multi-View Tab Switcher in Device Console

- [x] 2.1 Add tab bar header with `[ HARDWARE ]`, `[ REST API ]`, and `[ MCP AGENT ]` buttons in `firmware-rs/assets/device.html`
- [x] 2.2 Add view container styling and responsive layout for active/inactive panels
- [x] 2.3 Implement client-side URL hash and pathname detection on page load (`initView()`) and tab click handlers

## 3. Interactive REST API Catalog

- [x] 3.1 Construct structured endpoint catalog covering System/Telemetry, Pin Arbiter, Transceivers, Servo/PWM, and WebSockets
- [x] 3.2 Render endpoint rows with HTTP method badges, paths, descriptions, and 1-click `curl` copy
- [x] 3.3 Add interactive "Try Endpoint" modal or inline fetch runner with live HTTP status and JSON response viewer

## 4. MCP Agent Bridge & Schema Documentation

- [x] 4.1 Build MCP connection information card with `/mcp` HTTP endpoint and `/mcp/sse` event stream
- [x] 4.2 Add 1-click copyable `mcpServers.agentla` configuration snippet for Antigravity, Claude Desktop, and Cursor
- [x] 4.3 Document native on-probe tools (`mcp_target_diagnose`, `mcp_inspect_firmware`, `mcp_sniff_bus`, `mcp_pins_claim`, `mcp_telemetry_get`) with arguments and descriptions
- [x] 4.4 Add live JSON-RPC test execution box (e.g. testing `tools/list` over POST `/mcp`)

## 5. Verification & Testing

- [x] 5.1 Update `test/test_device_diagnostics.py` to assert tab switching elements, route aliases, REST catalog rows, and MCP config snippets
- [x] 5.2 Validate with `cargo check` and run pytest test suite
- [x] 5.3 Verify LittleFS packaging and ROM flash headroom with `tools/pack_webui.py`
