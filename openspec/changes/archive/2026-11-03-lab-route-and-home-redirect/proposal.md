## Why

Currently, the AgentLA workbench is served at the root URL (`/` and `/index.html`), while the multi-app launcher resides at `/apps`. Beyond routing, the current App Launcher visual styling diverges sharply from the workbench: `apps.html` contains hardcoded legacy GitHub dark mode styles (`#0d1117`), ignores the modular DaisyUI theme engine (`forest`, `emerald`, `caramellatte`), and completely lacks the System Settings modal, device actions (Reboot/BOOTSEL), and hardware telemetry present in the workbench. Furthermore, light mode is currently broken because the theme compiler emits the `:root` selector at the bottom of `themes.css`, cascade-overriding all light theme variables with the dark fallback, and `app.js` has a `localStorage` key mismatch. Third-party apps also lack a way to return to the launcher.

Unifying both pages under a single cohesive embedded OS model gives AgentLA a clean architecture:
- The core workbench moves to `/lab`.
- The root URL (`/`) redirects (via HTTP 302) to the user-configured landing page.
- Both `/lab` and `/apps` share the exact same clean OS top bar and shared System Settings modal (`#modal-settings`).
- Redundant top-bar navigation pills are eliminated, with the top-left `[❖ AgentLA]` brand acting as the portal to the App Launcher.
- The App Launcher is reimagined as an authentic OS-style app grid (tactile squircle tiles, spotlight search, segmented storage bar, tap-to-launch).
- Installed apps under `/apps/{app-name}/` automatically receive an injected circular glassmorphic floating action button (with the `win-rar-bold-duotone` Solar icon fixed at bottom-right) for 1-tap return to `/apps`.
- Theme compilation and `app.js` state handling are corrected so Light Mode (`emerald` and `caramellatte`) renders with clean, high-contrast, fully functional light styling across both workbench and launcher.

## What Changes

- **Dedicated Lab Route (`/lab` & `/lab/`)**:
  - Firmware HTTP server (`app_routing.c`) serves the factory workbench (`index.html`) on `/lab`, `/lab/`, and `/lab/index.html`.
  - Static assets resolve cleanly from root or `/lab/`.
- **Configurable Root 302 Redirection (`/`)**:
  - Requesting root `/` returns an HTTP 302 Found response pointing to `Location: <default_home_route>` (factory default: `/lab`).
  - Cached in RAM for microsecond zero-flash-read dispatch; persisted in LittleFS `/settings.json` via `/api/v1/settings`.
- **OS-Style App Launcher (`/apps`)**:
  - Replaces `apps.html`'s 350-line legacy foreign CSS with the unified design system (`style.css` + `themes.css`) mounted inside `.app-shell` and `#main-workspace`.
  - Shares the technical drafting grid background, fractal noise overlay, and DaisyUI tokens with the workbench.
  - **Spotlight Search**: Centered floating pill search bar (`/` hotkey) with real-time live filtering across app names, tags, and tools.
  - **Core Instruments & Flagship Section**: Tactile squircle app tiles for core instruments:
    - **AgentLA Lab**: Primary flagship app tile with a `FLAGSHIP` badge and direct launch into the workbench.
    - **MSO Scope**: Direct launch into Mixed-Signal Oscilloscope & Logic Analyzer mode.
    - **Pinout Matrix**: Direct launch into Hardware Pinout Matrix mode.
    - **USB Adapters**: Direct launch into Hardware Protocol Adapters mode.
    - **Console**: Direct launch into Firmware REPL & MCP console.
  - **Installed Apps Section**: Tactile squircle app tiles (64×64px squircle icons with rim highlights, titles, subtitles, hover uninstall buttons, and 1-tap direct launch into full viewport). Includes an inline **"Install App" (+)** tile for local package upload.
  - **OS Dock & Community Store**: Segmented capacity bar (System ROM, Installed LittleFS apps, Free Flash) in a footer dock, paired with a sliding segmented control to switch instantly into the searchable Community App Store.
  - **Demo Mode Resilience**: In static environments (GitHub Pages demo), seamlessly falls back to demo apps, mock LittleFS storage, and relative navigation (`index.html`) without 404 errors.
- **Clean OS Top Bar & Unification**:
  - Removes redundant "Workbench" and "App Launcher" navigation pills from both `/lab` and `/apps`.
  - In `/lab`, clicking `[❖ AgentLA]` navigates to `/apps`.
  - In `/apps`, clicking `[❖ AgentLA]` stays on the launcher, while launching the "AgentLA Lab" tile opens `/lab`.
  - Top bar on both pages features the identical utility suite: Target Power status (`3.31V · 14mA`), Emergency Safe State (``), System Settings gear (``), and 3-way Theme Toggle (`☼ / ☾ / ☲`).
- **Shared System Settings Modal (`#modal-settings`)**:
  - Both `/lab` and `/apps` embed and share `#modal-settings`.
  - Unified controls: Theme Mode (System / Light / Dark), Default Light Theme (`emerald` / `caramellatte`), Default Dark Theme (`forest`), Default Landing Page (`/lab` / `/apps` / installed apps), Reboot RP2350, Enter USB BOOTSEL, and live Hardware Telemetry.
- **Injected Floating Launcher FAB in Installed Apps**:
  - Firmware automatically appends `<script src="/agentla-bridge.js" defer></script>` to HTML responses served under `/apps/{app-name}/`.
  - Renders a 44×44px circular glassmorphic FAB fixed at bottom-right (`bottom: 16px; right: 16px; z-index: 99999`) inside an isolated closed Shadow DOM.
  - Contains the `win-rar-bold-duotone` Solar Icon SVG, with no text or extra buttons.
  - 1-tap navigates to `/apps`. Can be suppressed with `?no_launcher=1` or `window.AgentLANoLauncher = true`.
- **Light Mode & Theme Compiler Fix**:
  - `tools/compile_themes.py` emits `:root` as a standalone fallback at the very top of `themes.css`, ensuring specific theme selectors (`[data-theme="emerald"]`, `[data-theme="caramellatte"]`, `[data-theme-scheme="light"]`) properly override `:root` without cascade collisions.
  - `firmware/html/app.js` synchronizes both `agentla-theme-mode` and `agentla-theme` in `localStorage` and fallback paths.

## Capabilities

### New Capabilities
- `configurable-home-redirect`: HTTP 302 redirect on root `/` driven by in-memory cached setting, with REST API and WebUI configuration.
- `app-launcher-floating-fab`: Injected circular glassmorphic FAB with `win-rar-bold-duotone` icon in installed apps for 1-tap return to `/apps`.
- `unified-os-app-launcher`: Redesigned OS app launcher with tactile squircle grid, spotlight search, segmented storage, DaisyUI styling, clean top bar, and shared `#modal-settings`.

### Modified Capabilities
- `bench-theme-engine`: Corrected theme compiler `:root` emission order and `app.js` mode restoration so Light Mode functions properly.
- `url-routing-engine`: Updated to serve core workbench on `/lab`, redirect root `/` to the configured default route, and inject bridge script into `/apps/*` HTML responses.
- `adaptive-workspace-hub`: Updated top bar to remove redundant nav pills, link brand logo to `/apps`, and integrate landing page setting in modal.

## Impact

- `tools/compile_themes.py`: Emits `:root` fallback first before specific themes in `themes.css`.
- `firmware/src/app_routing.c`: Route `/lab` to workbench fsdata, 302 redirect `/` to `default_home_route`, append script injection to HTML responses under `/apps/`.
- `firmware/src/http_routes.c` / `firmware/src/api_routes.c`: Expose and persist `default_home_route` in `/api/v1/settings`.
- `firmware/html/apps.html`: Complete overhaul to OS-style launcher with DaisyUI tokens, squircle app grid, spotlight search, segmented storage, identical top bar, and `#modal-settings`.
- `web/agentla-bridge.js`: Shadow DOM component rendering circular glassmorphic FAB with `win-rar-bold-duotone` icon fixed at bottom-right.
- `firmware/html/index.html`: Clean top bar (remove redundant launcher link), set brand logo link to `/apps`, add landing page selector in `#modal-settings`.
- `firmware/html/app.js`: Wire settings controller for `default_home_route`, fix theme mode restoration keys.
- `tests/test_webui_assets.py` / test suite: Update URL path, theme cascade, and UI assertions.
