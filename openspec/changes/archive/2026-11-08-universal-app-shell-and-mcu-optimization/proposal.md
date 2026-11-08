## Why

User-installed LittleFS applications and built-in ROM instruments currently launch via top-level browser navigation (`window.location.href`). This architecture suffers from severe layout and performance drawbacks:
1. **Layout Obstruction**: The fixed floating glass dock sits directly on top of the bottom 84px-100px of the viewport, obscuring critical buttons, inputs, and controls in applications like CAN Sniffer and Servo Master.
2. **Top Bar Viewport Clipping**: Apps designed with `height: 100vh` or `overflow: hidden` are pushed down by the top bar, permanently clipping bottom controls below the display.
3. **Severe Microcontroller Overhead**: Every page transition tears down the DOM, forces the RP2350 MCU to re-read and stream heavy static assets (`style.css`, `daisyui.css`, `board.json`) from flash, and repeatedly drops and reconnects the `/api/v1/system/telemetry` SSE stream.

By establishing a zero-overhead persistent App-Shell architecture in `webui/index.html` with sandboxed iframe viewports, apps gain guaranteed top/bottom clearances, while the microcontroller experiences zero connection churn and minimal flash read wear.

## What Changes

- **Persistent App-Shell Viewport**: Add a dedicated `#view-app-viewport` container and iframe inside `webui/index.html` constrained between `top: 48px` (top bar) and `bottom: 84px` (dock clearance), ensuring no app elements are ever obscured.
- **Client-Side Hash Routing**: Implement clean URL hash routing (`#/`, `#/scope`, `#/terminal`, `#/files`, `#/app/:id`) enabling seamless back/forward navigation and deep linking without full-page reloads.
- **Embedded Chrome Suppression**: Auto-detect iframe execution (`is-embedded`) in `webui/agentla-bridge.js`, `webui/lab.html`, and `webui/terminal.html` to suppress duplicate top bars and docks.
- **Persistent Telemetry Session**: Maintain a single continuous `/api/v1/system/telemetry` SSE stream in the host shell across all application launches.
- **MCU HTTP Cache Optimization**: Update `firmware-rs/src/fs_http.rs` and `firmware-rs/src/web.rs` to serve immutable static assets (`.css`, `.js`, `.woff2`, `.svg`, `.json`) with long-lived client cache headers (`public, max-age=604800, immutable`), eliminating redundant flash reads.

## Capabilities

### New Capabilities
- `persistent-iframe-app-shell`: Universal single-page OS app-shell with persistent chrome, safe-area viewport container, and client-side hash routing.

### Modified Capabilities
- `unified-os-app-launcher`: Launch installed LittleFS applications inside the shell's persistent iframe viewport instead of `window.location.href`.
- `unified-javascript-bridge-sdk`: Detect iframe context and suppress redundant dock/top-bar mounting while providing zero-overhead parent API bridging.

## Impact

- **Web Assets**: `webui/index.html`, `webui/agentla-bridge.js`, `webui/lab.html`, `webui/terminal.html`, `apps/servo-master/index.html`, `apps/can-sniffer/index.html`.
- **Firmware**: `firmware-rs/src/fs_http.rs` and `firmware-rs/src/web.rs` static asset cache header tuning.
- **Breaking Changes**: None to API contracts. Apps running in the shell gain proper bounds and no longer require custom top-bar markup.
