## Context

AgentLA runs on an RP2350 microcontroller with strict `#![no_std]` zero-heap constraints. The `/device` page is served directly from Flash ROM (XIP) via `picoserve` and `include_str!("../assets/device.html")`, requiring zero dependencies on `SYSTEM_FS` or `USER_FS`. While `/device` currently focuses on hardware metrics (voltages, temperatures, dual A/B firmware slots, partition geometry, and pin ownership), users and AI agents frequently need quick access to REST API definitions and MCP connection details without booting desktop tooling or reading remote GitHub docs.

## Goals / Non-Goals

**Goals:**
- Provide instant, zero-friction discovery for AgentLA's headless capabilities (REST API and native on-probe MCP server).
- Support deep linking and subpath routing for `/device`, `/device/api`, and `/device/mcp` without creating multiple HTML files.
- Deliver 1-click copyable configuration snippets for Google Antigravity, Claude Desktop, and Cursor/OpenCode.
- Offer an in-browser interactive "Try Endpoint" fetch runner for immediate API testing.
- Maintain strict `#![no_std]` zero-heap operation with minimal Flash ROM delta (<8 KB).

**Non-Goals:**
- Embedding heavyweight 3rd-party OpenAPI/Swagger bundles (e.g. Swagger UI or Redoc, which require multi-megabyte JavaScript bundles).
- Runtime dynamic reflection of endpoints on the microcontroller (static declarative metadata in the console avoids microcontroller CPU and memory load).

## Decisions

### 1. Unified Single-Asset Multi-View Architecture (Option C)
- **Decision**: Extend `firmware-rs/assets/device.html` with a tab switcher and three view panels (`#hardware`, `#api`, `#mcp`), rather than compiling three separate HTML files.
- **Rationale**:
  - Eliminates ROM duplication: common CSS, header components, modal dialogs, and utility scripts are shared.
  - Tab switching occurs purely in the browser DOM in 0 ms with zero network round-trip.
  - Keeps total Flash footprint delta to ~8 KB, preserving ~193 KB of free firmware slot headroom.
- **Alternatives Considered**:
  - *Separate HTML files (`api.html`, `mcp.html`)*: Would duplicate 25+ KB of boilerplate and styles across three distinct ROM assets.
  - *Single continuous scrollable page*: Clutters hardware telemetry with long API tables, increasing visual fatigue during emergency diagnostics.

### 2. Subpath Aliasing in Picoserve Router (`web.rs`)
- **Decision**: Register `/device/api`, `/device/api/`, `/device/mcp`, and `/device/mcp/` in `firmware-rs/src/web.rs` as direct route aliases serving `crate::device_html::DEVICE_HTML`.
- **Rationale**:
  - Allows clean, intuitive bookmarking and deep linking from external tools and documentation.
  - Picoserve handles static routes with static string slices at compile time with zero heap allocation.
  - Client JavaScript detects `window.location.pathname` or hash on `DOMContentLoaded` and immediately reveals the intended tab.

### 3. Declarative Client-Side Catalog with Interactive Runner
- **Decision**: Store endpoint definitions and MCP tool signatures as a compact JavaScript structure inside `device.html`.
- **Rationale**:
  - Serving pre-baked schema arrays requires no microcontroller parsing or serialization.
  - Interactive "Try Endpoint" button uses native browser `fetch()` on the same origin (`window.location.origin`), completely sidestepping CORS issues and streaming results into a clean JSON viewer.

## Risks / Trade-offs

- **[Risk]** Slightly increased Flash binary size could threaten partition slot boundaries.
  - **Mitigation**: Current firmware text is 999 KB within a 1,200 KB slot. Adding 8 KB of uncompressed HTML text represents <0.6% of partition space, leaving ample headroom (>193 KB).
- **[Risk]** Navigation confusion between WebUI (`/home`, `/agent`) and In-ROM Console (`/device`).
  - **Mitigation**: Top navigation header provides explicit, high-contrast links back to the WebUI (`[ WebUI ]`) and between diagnostic views.
