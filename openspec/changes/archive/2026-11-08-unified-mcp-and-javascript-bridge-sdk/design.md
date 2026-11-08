## Context

AgentLA runs `#![no_std]` Rust firmware under the Embassy executor on a dual-core RP2350 (ARM Cortex-M33). It exposes a high-speed HTTP/WebSocket server (via Picoserve) over USB CDC-NCM Ethernet at `10.10.10.10`.
The system serves two audiences:
1. **Human Operators** viewing responsive WebUI apps (stored in the LittleFS `USER_FS` partition at `0x00270000+`).
2. **Autonomous AI Agents** communicating via Model Context Protocol (MCP JSON-RPC 2.0) over `POST /mcp` and `GET /mcp/sse`.

Currently, the MCP server is missing key dispatch arms (`catalog_lookup`, `programmer_unbrick`) and has no awareness of CAN bus, SPI, UART, or the Pin Arbiter. Meanwhile, frontend apps must manually construct complex `fetch` and WebSocket connections.

## Goals / Non-Goals

**Goals:**
- Deliver 100% functional parity between advertised MCP tools and firmware execution.
- Expose CAN-FD, SPI master, and UART terminal tools to autonomous AI agents over MCP.
- Enforce the Pin Arbiter safety contract within MCP via `pins_status`, `pins_claim`, and `pins_release`.
- Deliver `window.AgentLA` in `agentla-bridge.js`: a single-file, promise-based JavaScript SDK providing typed hardware access, WebSocket reconnects, offline simulation, and automatic safe-state on unload.
- Correct `/mcp.json` to be a compliant JSON autodiscovery manifest.
- Update `servo-master`, `can-sniffer`, and `terminal` to consume `window.AgentLA`.

**Non-Goals:**
- Altering the LittleFS partition layout or bootloader OTA slot addresses.
- Introducing npm/bundler build steps to `webui/` or `apps/` (must remain dependency-free vanilla JS).
- Dynamic runtime compilation of custom C/Rust firmware in MCP (tools interact with existing drivers).

## Decisions

### 1. Single Universal Script Distribution (`agentla-bridge.js`)
- **Decision**: Embed the complete `window.AgentLA` SDK inside `webui/agentla-bridge.js` alongside top bar, dock, and theme synchronization.
- **Rationale**: AgentLA apps are standalone LittleFS web pages or single-file dropped HTML files. Embedding the SDK in `agentla-bridge.js` allows apps to import one single `<script src="/agentla-bridge.js"></script>` with zero bundling or external CDNs required.
- **Alternatives Considered**:
  - Separate `agentla-sdk.js`: Requires apps to manage multiple script tags and handle loading order race conditions.
  - NPM module: Requires Vite/Webpack build steps, violating the zero-toolchain constraint for on-device app installation.

### 2. Automatic Safe-State on Tab Unload
- **Decision**: Hook `window.addEventListener('pagehide', ...)` and `beforeunload` to automatically trigger an asynchronous claim release / safe-state call.
- **Rationale**: Prevents hardware damage or motor runaway if a user closes a tab while servos are active or an external bus is held low.
- **Alternatives Considered**:
  - Requiring apps to manually call `detach()`: Prone to user error when closing or reloading the browser tab.

### 3. Unified CAN Bus Protocol Tools in MCP
- **Decision**: Provide four discrete CAN tools: `can_status`, `can_send`, `can_sniff`, and `can_set_termination`.
- **Rationale**: Allows an AI agent to inspect bus health (bitrate, TEC/REC, termination), transmit diagnostics (such as OBD-II PID queries or UDS commands), and capture traffic over bounded intervals without overflowing the `#![no_std]` 1536-byte output buffer.

### 4. Structured JSON Standardization in MCP Output
- **Decision**: All new and updated MCP tools format their output as compact JSON strings inside the MCP text response (`[{"type":"text","text":"{...}"}]`).
- **Rationale**: AI agents can reliably parse structured JSON fields rather than relying on brittle regex scraping of unstructured text (e.g. `clk=150MHz...`).

## Risks / Trade-offs

- **[Risk] Buffer limits on MCP response strings** → Firmware response buffers in `crates/agentla-mcp` are bounded (1536 bytes for tool output, 4096 bytes total).
  - *Mitigation*: Limit `programmer_read` to max 256 bytes per call, limit `can_sniff` to max 16 frames per capture, and use compact ASCII JSON formatting.
- **[Risk] Browser Beacon on Unload** → `fetch` during `beforeunload` may be cancelled by some browsers.
  - *Mitigation*: Use `navigator.sendBeacon('/api/v1/profiles/apply', JSON.stringify({slot: 0}))` with fallback to `fetch({keepalive: true})`.
- **[Risk] Pin Arbiter Contention between AI Agent and Web App** → An agent claiming pins while a web app is running.
  - *Mitigation*: The `agentla-pin-arbiter` crate enforces atomic driver ownership. If a pin is already claimed by `Servo`, `pins_claim` fails with `ClaimError::AlreadyClaimed` and reports the conflicting driver to the agent.
