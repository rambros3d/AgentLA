## Context

The MCP spec as of 2026 uses JSON-RPC 2.0 with two transports over HTTP:
- A POST-first request/response over `/mcp` (the older HTTP + SSE server-transport pairing)
- Streamable HTTP at `/mcp` with `Accept: text/event-stream` upgrades.
Claude Desktop and most clients default to the older `(POST /mcp + SSE GET /mcp/sse)` shape; both deliver identical JSON-RPC bodies.

Cold start: phase 2 already exposes `pin_owner`, `profile`, `servo`; Phase 1 exposes storage; phase 3 exposes capture state. The handlers for MCP tools mostly call these modules, so integration is mechanical.

## Goals / Non-Goals

**Goals**: 
- Handshake + tool listing + tool call + (v1) notifications
- 7-8 representative tools
- JSON-RPC framing with server-to-client channels over SSE
**Non-goals**:
- authorization / OAuth 2.1 (MCP spec's new piece) — basic Origin check enough for local-loop USB-tether Probed party
- multi-user state, sampling / roots callbacks (large, deferred)
- file/binary contents response (text only - JSON summaries)

## Decisions

1. **Transport pairing instead of protocol upgrade**: `(POST /mcp, req/json) → JSON single body` + `GET /mcp/sse → server-to-client emits`. Both coexist, simplest path.
2. **Input shape from JSON bodies**: Full body buffered up to 8 KiB then parsed with a minimal stack-held byte scanner (no serde alloc).
3. **Session ID token**: UUID-like 16-byte hex generated at SSE open time by `embassy_time` ms PRNG (not crypto-random; fine for local tether); holder registered it in a static Option.
4. **`tools/list` output**: each entry `{ name, description, inputSchema }`. `inputSchema` canned JSON string per tool.
5. **Tool result wrapping**: `{"content":[{"type":"text","text":"<structured JSON>"}],"isError":bool}` – per MCP spec.
6. **Engine graceful failure**: a tool handler that finds the subsystem `None` (e.g., filesystem not mounted, unclaimed pin) returns `isError:true` with explanation, never panics.
7. **Request ordering**: single FIFO (probe is single host); no concurrency admission.
8. **CORS**: restrict to loopback origin only. Limit probe to 10.10.10.x; if Origin not allowlisted, 403.

## Risks / Trade-offs

- [MCP clients may ping at 5 s overhead in USB + Wi-Fi] → no heavy timer call per latency; `system_reboot_bootsel` defers via 50 ms timer.
- [Some clients query server capabilities before tools] → sequential JSON-RPC await required.
- [JSON parser malformed] → return JSON-RPC `ParseError` (-32700) / `InvalidRequest` (-32600).
- [Device memory ~416 KB after bank] → keep input buffers small (8 KiB max). 

## Migration Plan

Non-breaking: all under `/mcp/*` and `/api/v1/mcp`; no modifications to Phase1-4 surfaces; no disk format changes.

## Open Questions

- Should we support the SI UE (streamable HTTP) S-I-dispatch? deferred to v2.
- Authorization tier: probe local-only (origin allowlist), no token (current design); add when MCP adoption grows.