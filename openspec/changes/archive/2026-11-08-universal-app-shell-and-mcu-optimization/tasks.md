## 1. App-Shell Container & Viewport Geometry

- [x] 1.1 Add `#view-app-viewport` and `#app-viewport-frame` iframe container to `webui/index.html` styled with `top: 48px`, `bottom: 84px`, and `height: calc(100vh - 132px)`
- [x] 1.2 Implement client-side hash routing engine in `webui/index.html` handling `#/`, `#/scope`, `#/terminal`, `#/files`, and `#/app/:id`
- [x] 1.3 Add active app title pill with close ('x') button to `#top-bar` in `webui/index.html` returning to the launcher when clicked
- [x] 1.4 Update squircle tile and dock button click actions to route through the shell viewport rather than performing `window.location.href`

## 2. Chrome Suppression & Iframe Delegation

- [x] 2.1 Update `webui/agentla-bridge.js` to detect `window.self !== window.top` and suppress duplicate `#top-bar` and `#floating-glass-dock` rendering
- [x] 2.2 Add `html.is-embedded` style rules to `webui/lab.html` and `webui/terminal.html` to hide redundant top bar and dock headers when hosted in the shell
- [x] 2.3 Verify `BroadcastChannel('agentla-theme-channel')` and `postMessage` theme synchronization between shell and embedded iframes

## 3. Installed App Clearance & Markup Cleanup

- [x] 3.1 Update `apps/can-sniffer/index.html` to remove duplicate `#top-bar` markup and let the frame fill the viewport cleanly with visible TX controls
- [x] 3.2 Update `apps/servo-master/index.html` to remove duplicate `#top-bar` markup and verify bottom sequencer controls are unobstructed
- [x] 3.3 Repack LittleFS WebUI image using `python3 tools/pack_webui.py`

## 4. Microcontroller HTTP Static Caching Optimization

- [x] 4.1 Update `firmware-rs/src/fs_http.rs` to serve static assets (`.css`, `.js`, `.woff2`, `.svg`, `.json`) with `Cache-Control: public, max-age=604800, immutable`
- [x] 4.2 Update `firmware-rs/src/web.rs` routes for static assets to eliminate `no-cache, no-store` on immutable web files
- [x] 4.3 Run automated tests with `cargo test` and `pytest test/` to verify firmware compilation, HTTP routing, and webui packing
