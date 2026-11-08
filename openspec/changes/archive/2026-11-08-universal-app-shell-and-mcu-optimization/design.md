## Context

In the current AgentLA WebUI architecture, launching an installed app or navigating to a system instrument performs a full page navigation (`window.location.href`). This tears down the page, drops active SSE telemetry and WebSocket streams, and triggers repeated downloads of large static CSS/JS assets from RP2350 flash memory. Furthermore, apps have no safe-area bounding, meaning the fixed floating glass dock (`bottom: 24px`) overlaps and occludes bottom controls, while the 48px top bar pushes `100vh` layouts off-screen.

Since backward compatibility is not required, this design re-architects AgentLA WebUI into a true single-page operating system (SPA OS) shell where `webui/index.html` hosts all applications in a dedicated safe-area iframe container.

## Goals / Non-Goals

**Goals:**
- Provide a persistent desktop shell with unified top bar and floating glass dock that never unloads or reconnects telemetry across app launches.
- Confine all application frames inside a safe viewport (`top: 48px`, `bottom: 84px`) so UI elements are never covered by the dock or clipped by the top bar.
- Implement client-side URL hash routing (`#/`, `#/scope`, `#/terminal`, `#/files`, `#/app/:id`) with browser back/forward history support.
- Eliminate duplicate top bar and dock rendering in embedded instruments via `is-embedded` / `window.self !== window.top` detection.
- Optimize microcontroller performance by caching immutable static assets with `Cache-Control: public, max-age=604800, immutable`.

**Non-Goals:**
- Multi-window tiling or overlapping desktop window managers (all apps run focused full-viewport).
- Re-architecting backend firmware IPC or protocol bridges (only static HTTP caching headers are updated).

## Decisions

### 1. Viewport Sandboxing & Geometry
- **Decision**: Embed applications inside `<iframe id="app-viewport-frame" class="app-viewport-frame">` housed in `<div id="view-app-viewport">`.
- **Dimensions**:
  ```css
  #view-app-viewport {
      position: fixed;
      top: 48px;
      bottom: 84px;
      left: 0;
      right: 0;
      width: 100%;
      height: calc(100vh - 132px);
      z-index: 10;
  }
  ```
- **Rationale**: An iframe creates an isolated DOM tree and viewport context. The app's `window.innerHeight` and CSS `100vh` are evaluated relative to `calc(100vh - 132px)`, making standard responsive layouts work without overlapping the floating dock.

### 2. Client-Side Hash Routing & Navigation
- **Decision**: Use URL hash fragments for navigation (`#/`, `#/scope`, `#/terminal`, `#/files`, `#/app/servo-master`).
- **Rationale**: URL hash changes do not trigger server requests to the RP2350 microcontroller. Deep links load the shell first, which automatically mounts the target app iframe based on the hash.

### 3. Parent-Child Theme & Bridge Coordination
- **Decision**: Child apps inherit themes via `BroadcastChannel('agentla-theme-channel')` and `postMessage({ type: 'THEME_CHANGE' })`.
- **Rationale**: This is already supported in `agentla-bridge.js`. When inside an iframe, `agentla-bridge.js` suppresses its own `#top-bar` and `#floating-glass-dock` rendering, eliminating duplicate DOM overhead.

### 4. Immutable Static Asset Caching on RP2350
- **Decision**: Configure `firmware-rs/src/fs_http.rs` to serve `.css`, `.js`, `.woff2`, `.svg`, and `.json` with `Cache-Control: public, max-age=604800, immutable`.
- **Rationale**: Browsers cache static bundles locally. Subsequent app switches and page loads require 0 flash reads on LittleFS.

## Risks / Trade-offs

- **[Risk] Cross-origin restrictions on iframes**: → *Mitigation*: Both the shell (`/`) and apps (`/app/:id/` or `/scope`) are served from the same origin (the AgentLA web server), allowing full DOM and API communication when needed.
- **[Risk] WebSocket disconnects when iframe changes src**: → *Mitigation*: The parent shell maintains persistent telemetry, while instrument-specific WebSockets (e.g. CAN/UART) naturally connect when the app opens and cleanly close on app switch.
