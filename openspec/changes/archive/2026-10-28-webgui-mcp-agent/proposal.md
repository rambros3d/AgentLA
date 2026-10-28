# WebGUI-MCP Agent — Detailed Plan

## Summary
Extend AgentLA from CLI probe to a probe-side network agent on RP2350, serving
MCP-style endpoints (JSON-RPC `/mcp`), SSE streams, and a PicoLab-style
WinBox desktop web GUI — all directly over USB-RNDIS/LwIP.

## Scope
- Probe-side on RP2350 (Pico 2)
- HTTP + SSE over RNDIS (no host proxy)
- WinBox /mcp / /sse endpoints replacing CLI
- Dual-core: Core 1 timing-sensitive; Core 0 network+UI

## Thorough Architecture (phased)
Phase 1 Network Foundation → 2 HTTP+Assets → 3 /mcp → 4 SSE → 5 WinBox UI → 6 Integration
Rationale: PicoLab reusable scaffolding (dhserver, dnserver, netif glue).

## Design Decisions (refined)
- USB interface: RNDIS + CDC composite (PicoLab scaffold, enables recovery)
- MCP domains (phase-first): Debug + Flash, Logic analyzer, GPIO
- Security: None by default (USB-local trusted)
- Small command surface and structured error responses

## Non-goals
- Full MCP spec compliance (hand-rolled JSON-RPC endpoints)
- Production-grade security (local trusted-USB default)

## Risks
- RNDIS MTU/latency vs raw USB. Cache carefully.
- Dual-core shared RAM races. Lock light.
- UI memory on flash: 2MB is plenty for WinBox.js+app.js.
- Backward: CLI removed — breaking for host scripts. Keep small `tty` fallback maybe in later phase.
