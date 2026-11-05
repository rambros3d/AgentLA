## Why

The Rust firmware currently exposes no way for an AI agent (Claude Desktop, antigravity, Goose, mcp-cli, etc.) to discover and invoke hardware tools. The C reference only ships an 8-line stub (`mcp_endpoint.c`), so this is effectively a net-new subsystem rather than a port. The MCP (Model Context Protocol) spec has matured into a JSON-RPC 2.0 / Server-Sent-Events flavor with a well-defined `initialize` handshake, `tools/list` discovery, `tools/call` execution, and `notifications/initialized` lifecycle. Implementing this in Rust again turns AgentLA from "web user only" into an AI-device, which is the stated product point, while the underlying tool surface (status, pins, profiles, servo, capture) already exists and just needs a JSON-RPC barrel.

## What Changes

- **New `mcp.rs` module** hosting a framed JSON-RPC 2.0 server over HTTP/SSE. Reuses the Phase-1 picoserve + SSE machinery (`/mcp/sse` for the notification stream, `/mcp/rpc` or `POST /mcp` for requests). Content-type `application/json` with `text/event-stream` for the SSE back channel.
- **Tool registry** with a small owned list of available tools: `status`, `probe_target`, `read_pins`, `capture_signals`, `i2c_scan`, `servo_attach`, `profiles_activate`, plus `system_reboot_bootsel`. Each has an input JSON-schema descriptor and a Rust closure that calls existing subsystem APIs.
- **Handshake**: `initialize` returns `{protocolVersion, serverInfo, capabilities:{tools:{}}, instructions}`. `notifications/initialized` is accepted (returns 204). `tools/list` enumerates the registry with name/description/input-schema. `tools/call` dispatches by name, serializes results as `{content:[{type:"text",text:...}], isError: false|true}` and surfaces handler errors as `isError:true` with structured `error_data`.
- **Session management** optional: a single-tenant session id embedded into the SSE handshake URL. Multi-session not required for v1 (the probe is currently single host-tethered). Logout/disconnect simply doesn't release anything unique since tools are stateless.
- **Rate limiting + safety**: safe-state cutoff before any pin-claiming tool; pin-claiming tool surface returns before hardware touch if a `claim` cannot be secured. Maximum payload 16 KiB (`JSON body cap`).
- **Health endpoint**: `{ "status": "healthy", "tools": N }` under `/api/v1/mcp` health route alongside the `/mcp` root, so the host side can probe liveness.
- **REST metadata**: `GET /api/v1/mcp` exposes the SSE URL and tool count so a web UI can display it.

## Capabilities

### New Capabilities
- `mcp-protocol-handshake`: initializes server/client contract and server info.
- `mcp-tool-dispatch`: registers, calls, serializes hardware tools.
- `mcp-authorization-safety`: guards (rate limiting + pin safety).

### Modified Capabilities
- `webui-mcp-api` (existing): narrows the Rust embodiment to the streaming-SSE + JSON-RPC dispatch shape; removes desktop-level hooks (the REST surface only).
- `webui-control-surface`: adds MCP toast/status indicator (out of scope of this change — noted, not part of tasks).

## Impact

- New module `firmware-rs/src/mcp.rs` (~400-600 LOC): framing, state machine, tool registry, handler table.
- ~6 web handlers added (`/mcp`, `/mcp/sse`, `/api/v1/mcp`, `/api/v1/mcp/health`, ...
- `programmer/pins.rs` + `pin_owner.rs`: for the safety path, query-only.
- No new dependencies: handshake requires a JSON encoder/decoder. The web stack already has `heapless` + manual `write!` helpers. A minimal JSMN-like parser is written in module (range-jump, no alloc).
- Flash usage: + ~70 KB.
- SRAM usage: ~10 KB (session, headers, input buffer).
- JSON-RPC response limits: 4 KiB streaming per chunk (SSE) so there's no 4 KB ceiling as Phase 1 has.