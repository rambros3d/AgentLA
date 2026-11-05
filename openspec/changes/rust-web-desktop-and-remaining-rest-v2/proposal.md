## Why

The Rust firmware already mounts LittleFS (Phase1) and serves dynamic app assets, but the desktop OS shell (index, theme/themes, sidebar docking, floating app launcher, app management) and several `/api/*` endpoints (theme state, board manifest, settings, target power, analog read) remain unported; the current response is a fallback page only. Restoring the full desktop is the last missing visible piece.

## What Changes

- serve `firmware/html/index.html` contents as embedded fallback (already Phase1)
- add `/api/settings`, `/api/board/manifest`, devicepower/vtarget telemetry, advanced pins
- extend *Phase5 shell* with optional subtle CSS for tag analytics
- `tar_unpacker` to LittleFS directly for user-dropped `.tar` bursts (in-progress Phase1 already basic).
- health via chrome manifest to identify board profile.

## Capabilities

### New
- `web-desktop-daisyui-shell`: desktop css, icons pre-cached, full JS.
- `board-management-poll`: board profile/current voltage/current telemetry.
- `webui-rest-settings`: theme/chrome/api/... setting reads.

### Modified
- `webui-control-surface`: dependency on desktop-only pages
- `rest-addressable-led` reads config by `board.json` extension.
