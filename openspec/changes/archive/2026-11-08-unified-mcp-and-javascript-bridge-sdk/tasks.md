## 1. MCP Server Tool Parity & Core Programmer Repair

- [x] 1.1 Wire `catalog_lookup` tool in `firmware-rs/src/mcp.rs` to call `agentla_targets::catalog::lookup_by_name()` and return structured JSON
- [x] 1.2 Wire `programmer_unbrick` tool in `firmware-rs/src/mcp.rs` to invoke `agentla_targets::engines::rescue::execute_unbrick()`
- [x] 1.3 Update `TOOL_PROGRAMMER_READ` schema in `crates/agentla-mcp/src/tools.rs` to accept address & length, and update `firmware-rs/src/mcp.rs` to parse them
- [x] 1.4 Implement and register `TOOL_PROGRAMMER_FLASH` in `agentla-mcp` and `firmware-rs/src/mcp.rs` to write binary memory chunks with verification
- [x] 1.5 Parameterize `TOOL_LOGIC_ANALYZER_ARM` and `TOOL_I2C_SCAN` with input parameters in `crates/agentla-mcp/src/tools.rs` and `firmware-rs/src/mcp.rs`

## 2. Automotive & Industrial MCP Protocol Tools

- [x] 2.1 Define `can_status`, `can_send`, `can_sniff`, and `can_set_termination` in `crates/agentla-mcp/src/tools.rs`
- [x] 2.2 Implement CAN tool execution in `firmware-rs/src/mcp.rs` using `crates/agentla-bridges/src/can.rs`
- [x] 2.3 Define and implement `spi_transfer` tool in `crates/agentla-mcp` and `firmware-rs/src/mcp.rs`
- [x] 2.4 Define and implement `uart_command` tool in `crates/agentla-mcp` and `firmware-rs/src/mcp.rs`

## 3. Pin Arbiter Safety & Structured Telemetry

- [x] 3.1 Define and implement `pins_status` MCP tool returning full GP0–GP29 allocation matrix and active driver claims
- [x] 3.2 Define and implement `pins_claim` and `pins_release` MCP tools enforcing atomic driver reservation in `agentla-pin-arbiter`
- [x] 3.3 Upgrade MCP `status` tool response from plain text to compact structured JSON

## 4. MCP Discovery & Universal Client Bridge SDK

- [x] 4.1 Update `webui/mcp.json` to return a compliant JSON discovery manifest with endpoints and capabilities
- [x] 4.2 Implement `window.AgentLA` in `webui/agentla-bridge.js` with modules: `system`, `pins`, `can`, `uart`, `i2c`, `spi`, `servo`, `programmer`, `mcp`, `telemetry`, and `ui`
- [x] 4.3 Add automatic safe-state pin release hook on `pagehide` and `beforeunload` in `agentla-bridge.js`
- [x] 4.4 Add in-memory virtual hardware simulator in `agentla-bridge.js` for demo mode parity

## 5. Default Applications Refactoring & Verification

- [x] 5.1 Refactor `apps/can-sniffer/index.html` to consume `AgentLA.can.openStream()`, `AgentLA.can.send()`, and `AgentLA.can.setTermination()`
- [x] 5.2 Refactor `apps/servo-master/index.html` to consume `AgentLA.servo` and `AgentLA.pins`
- [x] 5.3 Refactor `apps/terminal/index.html` to consume `AgentLA.uart.openTerminal()`
- [x] 5.4 Repack WebUI partition (`tools/pack_webui.py`), flash to RP2350 target, and verify live endpoints and SDK integration
