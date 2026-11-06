## Why

The remaining web-maturity gap: AgentLA has a working embedded single-page fallback UI, but the full DaisyUI desktop (apps, docking, themes, app store, tarball installer endpoint) and the remainder of the C `rest_api.c` surface (profiles/pins already Phase2; ADC, system telemetry, cascade routes) are unported. With Phase1 through Phase4 scaffolding complete, this closes the Rust branch's web experience loop so it matches the C firmware desktop affordances.

## What Changes

- **Web desktop port** (`firmware-rs/src/web_desktop.rs`): static single-page shell (already Phase 1) + extended CSS + DaisyUI classes pre-rendered; when `/` is requested, serve the LittleFS-mounted `index.html` if present else fall back to Phase 1 page.
- **Profile UI routes**: `GET /api/v1/profiles`, `/profiles/active`, ... already Phase2; add `GET /api/profiles/{id}` and `GET /api/profiles/settings` aliases for desktop home listing.
- **System telemetry endpoint**: `GET /api/v1/system` already Phase1; add `GET /api/v1/device/power` (voltage/temp mock) + `GET /api/v1/adc/{pin}/read`.
- **Cascade set of board-management routes**: `GET /api/board/manifest`, `GET /api/settings`, complete adapter routes (`/api/v1/adapter/{i2c,...}`).
- **WebSocket/SSE hybrid**: `/api/v1/events` already Phase1 for telemetry; add `ws` passthrough channel for Pi turbo captures (next phase's data payload comes as this binary channel).
- **Static asset cache headers**: set `Content-Type` and `Cache-Control: immutable` headers.

## Capabilities

### New capabilities
- `web-desktop-daisyui-shell`: shell and theme Granddog preset.
- `webui-mcp-rest-barrel`: REST discovery surface among host+probe.
- `webui-board-management`: board overlay route and device status.

### Modified capabilities
- `webui-app-launcher`: changes launcher registration endpoint payload shape to include `entry`, `size` fields (C parity).
- `webui-mcp-api`: already considers Rust; no change beyond spec. 
- `webui-control-surface`: theme + state labels.

## Impact

- New module `web_desktop.rs`, CSS single string; growing flash usage + ~100 KB.
- Web server stack per request guard unchanged.
- Pin arbitration unaffected.
- No net-new filesystem persistence. 
  - Front-end offline asset bundle install already covered by Phase1 app-installer.