## Why

The AgentLA app launcher lists a "CAN Bus Analyzer" in its catalog, but the app directory (`apps/can-sniffer` / `webui/apps/can-sniffer`) does not exist. Furthermore, default app seeding in `firmware-rs/src/apps.rs` hardcodes dummy placeholder HTML directly in firmware ROM flash, wasting precious dual-slot execution space (~1.2 MB limit per slot). 

By establishing a full-duplex WebSocket CAN bridge (`/api/v1/can/ws`), implementing the dedicated CAN Bus Analyzer web application, eliminating bloated static HTML from ROM flash, and synchronizing `webui/apps/` with the root `apps/` directory, AgentLA gains a complete automotive diagnostic workbench while keeping firmware ROM lean.

## What Changes

- **CAN Bus Analyzer Web Application (`/apps/can-sniffer/`)**: Add complete single-page application (`index.html`, `app.json`, `icon.svg`) matching AgentLA's Neumorphic dark theme, with real-time frame trace table, ID mask/whitelist filtering, arbitrary standard/extended/RTR frame injection, bitrate configuration (10k to 1M), and $120\,\Omega$ termination control.
- **Full-Duplex CAN WebSocket (`/api/v1/can/ws`)**: Implement an Embassy async WebSocket handler in `firmware-rs` streaming bidirectional CAN frames to the web interface with low latency.
- **Firmware ROM Decoupling**: Remove hardcoded HTML strings (`SERVO_MASTER_INDEX_HTML`, `TERMINAL_INDEX_HTML`) from `firmware-rs/src/apps.rs`. Seeding creates valid app directories and manifests, deferring the rich web bundles to the `SYSTEM_FS` LittleFS partition packaged by `tools/pack_webui.py`.
- **App Storage Directory Synchronization**: Ensure `apps/` and `webui/apps/` maintain identical copies of `servo-master`, `terminal`, and `can-sniffer`.

## Capabilities

### New Capabilities
- `can-bus-analyzer-application`: Dedicated interactive CAN 2.0B / CAN-FD packet inspector web app installed under `/apps/can-sniffer/` with real-time scrolling trace, ID filters, periodic/single frame injector, and bus physical layer controls.
- `can-websocket-bridge`: Full-duplex WebSocket endpoint at `/api/v1/can/ws` providing real-time streaming of incoming CAN frames and immediate bus transmission for outgoing frames.

### Modified Capabilities
- `first-boot-app-seeding`: Update boot seeding logic in `firmware-rs/src/apps.rs` to stop embedding static HTML strings in ROM flash, creating clean manifest references backed by `SYSTEM_FS`.

## Impact

- **Firmware Size & Memory**: Eliminates raw HTML string constants from `firmware-rs`, keeping firmware binary footprint within slot bounds.
- **REST / Web Routes**: Registers new WebSocket route `/api/v1/can/ws` alongside `/api/v1/terminal/ws`.
- **WebUI Bundle**: Packs `can-sniffer` into `webui.img` and `webui.uf2` through `tools/pack_webui.py`.
- **App Launcher**: Users can launch CAN Bus Analyzer directly from the desktop launcher and pinned dock.
