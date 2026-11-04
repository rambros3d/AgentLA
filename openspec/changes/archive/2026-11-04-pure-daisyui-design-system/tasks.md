## 1. Stylesheet & Token Overhaul

- [x] 1.1 Remove neumorphic tokens (`--card-neumorphic-shadow`, `--card-rim-highlight`), millimeter drafting grid background, and SVG noise overlay from `firmware/html/style.css`
- [x] 1.2 Implement pure DaisyUI elevation rules (flat resting `base-200` with 1px `base-300` border, subtle hover elevation)
- [x] 1.3 Add standard DaisyUI component classes (`.navbar`, `.card-border`, `.stats`, `.stat`, `.table-zebra`, `.mockup-code`, `.progress`) in `firmware/html/style.css`

## 2. Dynamic Waveform Canvas Token Binding

- [x] 2.1 Implement dynamic CSS property resolution helper in `firmware/html/app.js` caching palette colors
- [x] 2.2 Bind Oscilloscope canvas (`#scope-canvas`) background, reticle grid, and traces to dynamic theme tokens
- [x] 2.3 Bind Logic Analyzer canvas (`#logic-canvas`) and Signal Generator preview (`#gen-preview-canvas`) to dynamic theme tokens
- [x] 2.4 Hook theme change listener to automatically redraw active canvases on theme switch

## 3. Workbench (/lab) Component Modernization

- [x] 3.1 Upgrade top header to pure flat DaisyUI `.navbar` with `.navbar-start`, `.navbar-end`, and `.status` indicators
- [x] 3.2 Refactor oscilloscope telemetry and measurement chips to DaisyUI `.stats .stat` (`.stat-title`, `.stat-value`, `.stat-desc`)
- [x] 3.3 Refactor protocol decoder traffic log to DaisyUI `.table .table-zebra .table-pin-rows .table-sm`
- [x] 3.4 Refactor terminal output to DaisyUI `.mockup-code`
- [x] 3.5 Refactor instrument control groupings to DaisyUI `.join`, `.range`, and `.badge`

## 4. OS App Launcher (/apps) Modernization

- [x] 4.1 Update top bar to pure flat DaisyUI `.navbar` matching `/lab`
- [x] 4.2 Standardize squircle app cards with flat `base-200` containers, 1px `base-300` borders, and DaisyUI `.badge` tags
- [x] 4.3 Update storage capacity meter to DaisyUI `.progress .progress-primary`
- [x] 4.4 Ensure `#modal-settings` and `#install-modal` use DaisyUI `.modal` structure

## 5. Public Landing Page (web/) Parity

- [x] 5.1 Update `web/index.html` to use DaisyUI `.navbar`, `.hero .hero-content`, `.card`, and `.footer`
- [x] 5.2 Align `web/landing.css` tokens with `firmware/html/style.css` flat border and surface variables

## 6. Verification & Firmware Build

- [x] 6.1 Update and run automated asset tests (`pytest test/test_webui_assets.py test/test_multi_app_store.py`)
- [x] 6.2 Build firmware binary (`make -C build-work -j4 AgentLA`) to verify flash footprint and zero compilation errors
- [x] 6.3 Verify theme switching across Forest, Emerald, and CaramelLatte in both `/lab` and `/apps`
