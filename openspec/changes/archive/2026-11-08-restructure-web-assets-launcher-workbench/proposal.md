## Why

Web assets currently have mismatched names compared to their architectural roles: `apps.html` served the primary launcher/catalog, while `index.html` served the workbench SPA. Standard web conventions, tooling, static site hosts, and developers expect `index.html` to represent the root landing page/catalog (`/home` and `/`). Renaming `apps.html` -> `index.html` and renaming the workbench SPA to `lab.html` aligns filenames directly with their roles, eliminates indirection, preserves 0 ms in-memory waveform retention across instrument views, and requires no backward compatibility aliases.

## What Changes

- **Asset Renaming & Restructuring**:
  - `webui/apps.html` is renamed to `webui/index.html` (App Launcher & Catalog).
  - Existing `webui/index.html` is renamed to `webui/lab.html` (Unified Workbench SPA).
  - `webui/terminal.html` remains `webui/terminal.html` (Standalone Serial Terminal).
- **Firmware Route Dispatch (`firmware-rs/src/web.rs`)**:
  - Requests to `/home` and `/home/` serve `index.html`.
  - Direct requests to `/index.html` serve `index.html`.
  - Requests to `/scope`, `/debug`, `/adapter`, `/pins`, `/agent` serve `lab.html`.
  - Direct requests to `/lab.html` serve `lab.html`.
  - Requests to `/terminal` and `/terminal/` serve `terminal.html`.
  - Direct requests to `/terminal.html` serve `terminal.html`.
- **Internal Cross-Link Updates**:
  - Update internal anchors and navigation links in `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/app.js`, and `webui/mock-bridge.js`.
- **Static Portal Pipeline & Tests**:
  - Update `tools/build_portal.py` and `website/index.html` iframe embed to target `demo/lab.html?embed=1` or canonical `/demo/scope`.
  - Update Python unit tests (`test/test_webui_assets.py`) to validate `index.html` (Launcher) and `lab.html` (Workbench).

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `url-routing-engine`: Update factory WebUI serving invariants to route `/home` to `index.html` and workbench instrument routes (`/scope`, `/debug`, `/adapter`, `/pins`, `/agent`) to `lab.html`.

## Impact

- `webui/` asset directory files (`index.html`, `apps.html`, `lab.html`, `app.js`, `mock-bridge.js`, `terminal.html`).
- `firmware-rs/src/web.rs` route matcher.
- `tools/build_portal.py` and `website/index.html`.
- `test/test_webui_assets.py`.
