## Context

The AgentLA WebUI provides both an App Launcher & Catalog (managing built-in apps and installed community LittleFS apps) and a Unified Workbench Single-Page Application (SPA) containing logic analyzer, oscilloscope, protocol analyzer, target programmer, GPIO control, and MCP agent console.

Previously, `apps.html` held the launcher/catalog, while `index.html` held the workbench SPA. This inversion caused conceptual confusion, mismatched standard static hosting expectations (where `index.html` is the catalog / root page), and complicated internal relative links.

## Goals / Non-Goals

**Goals:**
- Rename `apps.html` to `index.html` (the App Launcher & Catalog).
- Rename the workbench SPA to `lab.html`.
- Keep `terminal.html` as the standalone Serial Terminal.
- Update `firmware-rs/src/web.rs` routing:
  - `/home`, `/home/`, `/index.html` -> `index.html`.
  - `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, `/lab.html` -> `lab.html`.
  - `/terminal`, `/terminal/`, `/terminal.html` -> `terminal.html`.
- Maintain 0 ms in-memory waveform retention when navigating between instrument tabs inside `lab.html`.
- Zero flash overhead in `#![no_std]` firmware and LittleFS.
- Update all internal cross-references across `webui/` assets, static demo iframe in `website/index.html`, and `test/test_webui_assets.py`.

**Non-Goals:**
- Backward-compatibility aliases for deprecated routes (`/lab`, `/apps`, `/target`, etc.) are explicitly out of scope.
- Multi-page architecture for individual instrument views (e.g., `scope.html`, `debug.html`) with SharedWorkers is excluded due to iOS Safari incompatibility, 60 FPS GC churn, and iframe COOP/COEP isolation issues.

## Decisions

### 1. In-Place Swap of HTML Entry Points
- `webui/apps.html` becomes `webui/index.html`.
- `webui/index.html` becomes `webui/lab.html`.
- **Rationale**: Direct alignment with industry standards: `index.html` serves root and catalog dashboard, while `lab.html` serves the integrated engineering workbench.

### 2. Firmware Route Mapping
- In `firmware-rs/src/web.rs`:
  - `path == "/home" || path == "/home/" || path == "/index.html"` maps to `serve_static(req, lfs, "index.html", ...)`.
  - `path == "/scope" || path == "/debug" || path == "/adapter" || path == "/pins" || path == "/agent" || path == "/lab.html"` maps to `serve_static(req, lfs, "lab.html", ...)`.
  - `path == "/terminal" || path == "/terminal/" || path == "/terminal.html"` maps to `serve_static(req, lfs, "terminal.html", ...)`.
- **Rationale**: Minimal branch logic in `no_std`, zero runtime heap, instantaneous O(1) matching.

### 3. Iframe Embed in Marketing Portal
- `website/index.html` embeds `<iframe src="demo/lab.html?embed=1">` (or relative path in staged `_site/demo/lab.html?embed=1`).
- `tools/build_portal.py` copies the renamed files cleanly without stale references.

## Risks / Trade-offs

- [Risk: Stale asset links or cache in browser] -> Mitigation: In dev/production, files are served with proper MIME types; relative anchor links (`/home`, `/scope`, etc.) route directly to canonical paths without relying on filename heuristics.
- [Risk: LittleFS pack tool missing renamed files] -> Mitigation: `tools/pack_webui.py` crawls `webui/` dynamically; verified with `test/test_webui_assets.py`.
