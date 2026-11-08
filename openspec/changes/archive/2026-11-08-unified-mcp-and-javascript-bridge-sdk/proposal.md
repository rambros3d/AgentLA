## Why

AgentLA serves as both a human-interactive hardware workbench (via WebUI apps) and an autonomous AI agent peripheral (via MCP JSON-RPC 2.0). While core hardware drivers exist in `#![no_std]` domain crates, the external control interfaces suffer from critical parity and developer experience gaps:
1. Advertised MCP tools (`catalog_lookup`, `programmer_unbrick`) fail with `unknown tool` due to missing firmware dispatch matches, and several tools have empty input schemas with hardcoded parameters.
2. Major hardware capabilities (CAN bus, SPI, UART, and Pin Arbiter safety) are completely inaccessible to AI agents via MCP.
3. `/mcp.json` serves non-JSON `<!--#MCP-->`, breaking standard MCP client discovery.
4. User web apps duplicate dozens of lines of ad-hoc WebSocket/fetch boilerplate without automatic reconnection, error resilience, or pin teardown safety.

This change delivers full MCP tool parity, introduces automotive/industrial protocol tools, fixes discovery, and embeds a unified `window.AgentLA` JavaScript client SDK with automatic safe-state pin release.

## What Changes

- **MCP Tool Parity & Fixes**:
  - Wire `catalog_lookup` to `agentla-targets::catalog` for part number and DevID queries.
  - Wire `programmer_unbrick` to `agentla-targets::engines::rescue` across STM32, Nordic, NXP, and Microchip.
  - Parameterize `programmer_read` with address and length schemas.
  - Add `programmer_flash` tool to flash target memory chunks.
  - Parameterize `logic_analyzer_arm` (channels, sample rate, trigger) and `i2c_scan` (SDA/SCL pins).
- **Automotive & Industrial Protocol Expansion**:
  - Add MCP tools for CAN bus: `can_status`, `can_send`, `can_sniff`, `can_set_termination`.
  - Add `spi_transfer` tool for full-duplex SPI master exchanges.
  - Add `uart_command` tool for target serial console commands.
- **Pin Arbiter & Telemetry Safety**:
  - Add `pins_status` MCP tool returning the 30-pin allocation matrix and active driver claims.
  - Add `pins_claim` and `pins_release` tools enforcing the pin ownership contract.
  - Upgrade MCP `status` tool to return structured JSON telemetry.
- **MCP Discovery & Client Bridge SDK**:
  - Correct `/mcp.json` to return a standard JSON discovery manifest with endpoints and capabilities.
  - Embed `window.AgentLA` in `webui/agentla-bridge.js` providing promise-based APIs for CAN, UART, I2C, SPI, Servos, Pins, Telemetry, and in-browser MCP calls.
  - Implement automatic safe-state pin teardown on `pagehide` / `beforeunload`.

## Capabilities

### New Capabilities
- `unified-javascript-bridge-sdk`: Universal browser client SDK (`window.AgentLA`) embedded in `agentla-bridge.js` providing typed APIs for all hardware peripherals, resilient WebSocket management, offline virtual simulation, and automatic safe-state teardown.
- `mcp-protocol-bridges`: MCP tools for CAN bus (`can_send`, `can_sniff`, `can_status`, `can_set_termination`), SPI master (`spi_transfer`), and UART terminal (`uart_command`).
- `mcp-pin-arbiter-safety`: MCP tools for querying pin crossbar claims (`pins_status`), acquiring driver ownership (`pins_claim`), releasing pins (`pins_release`), and structured JSON system/power telemetry.

### Modified Capabilities
- `mcp-protocol-surface`: Add missing dispatch matches for `catalog_lookup` and `programmer_unbrick`, parameterize input schemas for `programmer_read`, `logic_analyzer_arm`, and `i2c_scan`, and add `programmer_flash`.
- `webui-control-surface`: Replace stubbed `<!--#MCP-->` in `/mcp.json` with a standard MCP discovery manifest, and refactor default apps (`can-sniffer`, `servo-master`, `terminal`) to use `window.AgentLA`.

## Impact

- **Firmware (`firmware-rs`)**:
  - `firmware-rs/src/mcp.rs`: dispatch loop wiring for catalog, unbrick, CAN, SPI, UART, and Pin Arbiter.
  - `firmware-rs/src/web.rs`: verification of `/mcp.json` static serving and WebSocket bridges.
- **Domain Crates**:
  - `crates/agentla-mcp`: tool definitions and schema definitions in `tools.rs`.
- **WebUI & Applications**:
  - `webui/agentla-bridge.js`: implementation of `window.AgentLA` SDK and unload hook.
  - `webui/mcp.json`: standard JSON MCP discovery manifest.
  - `apps/can-sniffer`, `apps/servo-master`, `apps/terminal`: updated to consume `window.AgentLA`.
