# Design

## Context

See `proposal.md` for background and motivation.

AgentLA exposes both network HTTP REST endpoints (served via picoserve under Embassy async) and native high-speed hardware USB interfaces:
- Debug Probes: CMSIS-DAP v1/v2, ST-Link v2, and WCH-Link
- Bus Adapters: `i2c-tiny-usb` (I2C), `FTDI MPSSE` (SPI), `agentla-binary-bulk` (MIPI I3C), `gs_usb` (CAN), and `CDC-ACM` (UART)

Invoking bitbang SWD or synchronous bus master operations inside HTTP task handlers creates Embassy executor latency spikes, lacks DMA/FIFO hardware flow control, and causes pin contention whenever USB probe or adapter endpoints are concurrently connected.

## Goals / Non-Goals

**Goals:**
- Provide compile-time toggles (`HTTP_PROGRAMMER_ENABLED` and `HTTP_ADAPTERS_ENABLED`) defaulting to `false` in `firmware-rs/src/rest.rs`.
- Return HTTP 200 with structured warning JSON payloads (`status: "disabled"`, `use: "usb"`) on all `/api/v1/programmer/*`, `/api/v1/bridge/*`, and `/api/v1/bus/*` endpoints.
- Update `crates/agentla-mcp` resources (`agentla://skills/programmer`, `agentla://skills/dev`), prompts (`program_target`, `dev_workflow`), and tool implementations (`programmer_flash`, `programmer_read`, `programmer_unbrick`) to instruct agents to use direct host USB interfaces.
- Align MCP tool count at 58 tools across `rest.rs`, `mcp.json`, and test fixtures.

**Non-Goals:**
- Removing route paths from `web.rs`: keeping routes registered ensures backward compatibility, preventing unhandled 404 errors and returning actionable instructions.
- Removing underlying driver crates (`agentla-programmer`, `agentla-bridges`): they continue powering the USB hardware endpoints.

## Decisions

### 1. Structured Warning Payloads over HTTP 501 / 404
- **Decision**: Return HTTP 200 with `{ "status": "disabled", "warning": "...", "message": "...", "use": "usb", ... }`.
- **Rationale**: Web UI applications, fetch clients, and LLM agent tool callers handle 200 OK responses with structured JSON reliably. Dropping routes (404) or throwing 501 causes generic exceptions without actionable guidance.
- **Alternatives Considered**: HTTP 501 Not Implemented (breaks clients expecting standard JSON envelopes) or removing routes entirely (leads to silent 404 failures).

### 2. Compile-Time Build Gates in `rest.rs`
- **Decision**: Define `pub const HTTP_PROGRAMMER_ENABLED: bool = false;` and `pub const HTTP_ADAPTERS_ENABLED: bool = false;`.
- **Rationale**: Enables zero-overhead dead-code pruning or single-line re-enablement when native testing or debugging requires it.

### 3. Agent Tooling Alignment via MCP
- **Decision**: Update MCP skills and prompt templates to explicitly instruct agents to use host USB tools (`probe-rs`, `pyocd`, `openocd`, `sdk/agentla/i3c.py`) for flashing and high-speed bus transfers, while using MCP for board-level state (power, profiles, pin ownership, and logic analysis).
- **Rationale**: Agents interacting via MCP receive clear guidance without hallucinating HTTP flashing loops.

## Risks / Trade-offs

- **[Risk] Existing scripts using HTTP programmer/bus endpoints fail to perform transactions.**  
  → **Mitigation**: The response JSON includes `"use": "usb"` and lists exact compatible USB adapters/modes (`["cmsis", "stlink", "wch"]`, `["i2c-tiny-usb", "ftdi-mpsse", "agentla-binary-bulk", "gs_usb", "cdc-acm"]`).
- **[Risk] Web UI dashboards displaying bus scan or flash controls show disabled status.**  
  → **Mitigation**: Web UI can inspect `status === "disabled"` and present a prompt directing the user to connect via USB.
