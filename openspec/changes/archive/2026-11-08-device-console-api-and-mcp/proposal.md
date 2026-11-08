## Why

Developers, automation engineers, and AI agent developers currently lack an immediate, zero-friction discovery surface for AgentLA's headless capabilities (native REST API and on-probe MCP JSON-RPC server) when connected to the probe. By extending the self-contained In-ROM `/device` diagnostics console into a unified tabbed console featuring dedicated **REST API Catalog** and **MCP Agent Bridge** tabs (with deep-link aliases at `/device/api` and `/device/mcp`), developers and AI agents gain instant access to copyable connection configurations, on-probe tool schemas, endpoint paths, and curl snippets—with zero MCU heap overhead and less than 0.6% Flash ROM delta.

## What Changes

- Add a top-level tabbed navigation bar inside `device.html`: `[ HARDWARE ]`, `[ REST API ]`, and `[ MCP AGENT ]`.
- Implement client-side tab switching with URL hash sync (`#hardware`, `#api`, `#mcp`) and deep-linking support.
- Register route aliases in `firmware-rs/src/web.rs`: `/device/api`, `/device/api/`, `/device/mcp`, `/device/mcp/` serving `crate::device_html::DEVICE_HTML`.
- Add the **MCP Agent Bridge** view:
  - Display HTTP endpoint (`http://192.168.7.1/mcp`, POST JSON-RPC 2.0) and SSE stream (`/mcp/sse`).
  - Provide a 1-click copyable configuration block for Google Antigravity, Anthropic Claude Desktop, and Cursor/OpenCode.
  - List all on-probe native firmware tools (`mcp_target_diagnose`, `mcp_inspect_firmware`, `mcp_sniff_bus`, `mcp_pins_claim`, `mcp_telemetry_get`) with parameter signatures and descriptions.
  - Include an interactive JSON-RPC request tester directly in the browser.
- Add the **REST API Catalog** view:
  - Comprehensive categorized table of firmware endpoints: System & Telemetry, Pin Arbiter, Peripheral Bridges (CAN, UART, I2C/I3C, SPI), Servo/PWM, and WebSocket streams.
  - Method badges (GET, POST, DELETE, WS), route paths, request payloads, and 1-click `curl` command copy.
  - Interactive "Try Endpoint" fetch runner with live response status and JSON output.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `rom-device-diagnostics`: Extend requirements to specify the multi-view tabbed architecture, route aliases (`/device/api`, `/device/mcp`), interactive REST API catalog, and MCP Agent Bridge integration.

## Impact

- **Firmware ROM**: ~8 KB addition to compiled `device.html` in XIP Flash (remains under 1007 KB with >193 KB free headroom in 1200 KB slot).
- **MCU RAM / Heap**: 0 bytes dynamic allocation (`#![no_std]` static slice serving).
- **HTTP Routing (`web.rs`)**: 4 new route entries in Picoserve router.
- **Filesystem**: 0 bytes LittleFS usage; runs completely independent of `SYSTEM_FS` and `USER_FS`.
- **Test Suite**: Updates to `test/test_device_diagnostics.py` verifying tab rendering, deep-link routing, and endpoint/tool schemas.
