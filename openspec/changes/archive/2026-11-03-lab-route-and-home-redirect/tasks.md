## 1. Theme Compiler & Light Mode Cascade Fix

- [x] 1.1 Update `tools/compile_themes.py` to emit `:root` as an initial fallback block at the top of `themes.css` before any specific `[data-theme="..."]` rules, and recompile `firmware/html/themes.css`.
- [x] 1.2 Fix `firmware/html/app.js` to correctly check `localStorage.getItem('agentla-theme-mode')` on fallback and keep light mode state synchronized.

## 2. Firmware URL Routing & Redirection Engine

- [x] 2.1 Support `/lab`, `/lab/`, and `/lab/index.html` in `app_routing.c` by serving the factory WebUI workbench from ROM fsdata.
- [x] 2.2 Implement HTTP 302 Found redirection for root `/` pointing to the in-RAM cached `default_home_route` (defaulting to `/lab`).
- [x] 2.3 Implement HTML bridge script injection (`<script src="/agentla-bridge.js" defer></script>`) in `app_routing.c` for `.html` files served under `/apps/{app-name}/...`.

## 3. Settings REST API & Persistence

- [x] 3.1 Add `default_home_route` field to settings structure, in-RAM caching, and load from LittleFS `/settings.json` on boot.
- [x] 3.2 Expose `default_home_route` in `GET /api/v1/settings` and handle updates in `PUT /api/v1/settings`.

## 4. Injected Floating Action Button Asset

- [x] 4.1 Create `web/agentla-bridge.js` mounting a closed Shadow DOM container fixed at bottom-right (`bottom: 16px; right: 16px; z-index: 99999`).
- [x] 4.2 Embed the `win-rar-bold-duotone` Solar Icon SVG with multi-level duotone opacity inside a 44×44px circular glassmorphic FAB button.
- [x] 4.3 Add click navigation to `/apps`, and support suppression via `?no_launcher=1` query param or `window.AgentLANoLauncher === true`.

## 5. Unified OS-Style App Launcher (`apps.html`)

- [x] 5.1 Strip out legacy `#0d1117` CSS from `firmware/html/apps.html` and adopt `style.css` + `themes.css` DaisyUI tokens and noise/depth.
- [x] 5.2 Build OS-style tactile squircle app grid with direct tap-to-launch, hover spring elevation, and flagship "AgentLA Lab" card pointing to `/lab`.
- [x] 5.3 Add centered spotlight search bar with real-time fuzzy filtering across apps, tags, and descriptions.
- [x] 5.4 Implement macOS/iOS style segmented flash storage capacity bar (System ROM, installed apps, free LittleFS).
- [x] 5.5 Standardize top bar to match workbench: brand logo `[❖ AgentLA]`, Target Power status, Safe State button, Settings gear `[⚙]`, and 3-way Theme Toggle `[☼/☾]`, omitting redundant nav pills.
- [x] 5.6 Embed and bind the `#modal-settings` System Settings modal in `apps.html` with full feature parity (themes, default landing page, reboot, BOOTSEL, telemetry).

## 6. Workbench Navigation & Settings UI (`index.html`, `app.js`)

- [x] 6.1 Remove redundant "App Launcher" navigation pill from top bar in `firmware/html/index.html`.
- [x] 6.2 Set top-left brand logo link (`#brand-logo`) to navigate directly to `/apps`.
- [x] 6.3 Add "Default Landing Page" dropdown in `#modal-settings` in `index.html` and populate dynamically with `/lab`, `/apps`, and installed LittleFS apps in `app.js`.
- [x] 6.4 Wire Settings load and save to sync `default_home_route` with `PUT /api/v1/settings`.

## 7. Verification, Firmware Build, Commit & Push

- [x] 7.1 Update and run test suite (`pytest`) verifying `/lab` route, `/` 302 redirect, script injection, launcher assets, settings endpoints, and light mode cascade.
- [x] 7.2 Compile firmware target (`make -C build-work -j4 AgentLA`) and verify clean build with zero warnings or errors.
- [x] 7.3 Commit all changes with a descriptive message and push to `origin/dev`.
