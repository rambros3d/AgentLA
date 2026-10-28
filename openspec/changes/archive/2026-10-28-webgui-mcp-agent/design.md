# Technical Design

## Context
Existing AgentLA is CLI firmware on RP2040/Pico. This design extends to RP2350 with probe-side network/MCP + Winbox GUI.

## Goals / Non-Goals
**Goals:** Probe-side network, dual-core, HTTP+SSE, JSON-RPC /mcp, WinBox UI.
**Non-Goals:** Host-side proxy, full MCP spec, production security.

## Decisions
- **MCU target**: RP2350 for RAM/headroom.
- **Core split**: Core 1 capture/decode; Core 0 lwIP+HTTP+UI.
- **Network stack**: PicoLab scaffold (dhserver, dnserver, tusb rndis, lwip init/netif/arp/dns).
- **HTTP server**: pico_lwip_http; content served from compile-time embedded files via pico_httpd_content.
- **Dynamic vs static endpoints**: Static GET / returns HTML; POST /mcp executes JSON-RPC; GET /sse subscribes; others return JSON.
- **Web GUI**: Vanilla JS + WinBox.js; tool windows call backend via fetch + SSE. Reuse PicoLab list items/dialogs & stylings.
- **State**: debug session state kept in Core 1 (shared via mutex); flash progress + capture stream pushed by Core 1 via global flag + Core 0 SSE tx.
- **Memory budgets**: Flash 2MB, paginated HTML assets; use HTTP filesystem-like tiny overhead.
- **USB-composite**: RNDIS + CDC for web+serial recovery.
- **MCP tools scope**: Debug/Flash, LA, GPIO.
- **Auth/security**: None by default (USB-local; update later).

## Risks & Mitigation
- [RNDIS latency] HTTP context switches impair timing capture. → Core 0 only; capture is Core 1.
- [Dual-core sync] shared ring buffer can race. → Atomic flags + per-direction spinlock.
- [Memory] HTML + app.js big; reuse CMake pico_httpd_content conversion.
- [Bootloader handoff] keeping CDC (TinyUSB composite) may allow recovery if network fails.

## Migration plan
Incremental phases. Existing CLI code remains runnable until Web layered on top; then remove CLI output.
## Open Questions
Security model: CORS? WSS? Bootloader recovery via /mcp update.
