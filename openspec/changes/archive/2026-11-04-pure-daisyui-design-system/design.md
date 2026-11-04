# Design: Pure DaisyUI Design System & Component Revamp

## Context

AgentLA currently exhibits stylistic divergence across its surface areas:
1. **Workbench (`/lab`)**: Contains lingering neumorphic artifacts including dual-layer heavy drop shadows (`--card-neumorphic-shadow`), white inset specular highlights (`--card-rim-highlight`), a background millimeter engineering drafting grid, and a fractal SVG noise overlay. These artifacts distort light themes (`emerald`, `caramellatte`) and wash out boundaries.
2. **Instrument Canvases (DSO & Logic Analyzer)**: Render using HTML5 Canvas 2D with hardcoded hex colors (`#0d1117`, `#00e5ff`, etc.), creating a visual disconnect when switching between light and dark themes.
3. **OS App Launcher (`/apps`)**: Features tactile squircle cards but still references neumorphic variables and frosted blur headers.
4. **Public Marketing Website (`web/`)**: Relies on a separate custom CSS stylesheet (`web/landing.css`) rather than shared DaisyUI tokens.

By modernizing the entire frontend ecosystem with pure DaisyUI v5 conventions, we eliminate thousands of lines of fragile bespoke CSS, unify color management through CSS OKLCH tokens, and establish a clean, flat, high-contrast engineering aesthetic.

## Goals / Non-Goals

### Goals
- **Eliminate Neumorphism & Noise**: Completely remove dual-source neumorphic drop shadows, inset specular rim bevels, background millimeter CAD drafting grids, and fractal noise textures.
- **Pure DaisyUI Surface Hierarchy**: Enforce flat `base-100` page backgrounds, flat `base-200` card surfaces, and crisp 1px `base-300` borders (`border-base-300`), with gentle ambient elevation on hover.
- **Dynamic Canvas CSS Token Binding**: Bind HTML5 Canvas 2D render loops for the DSO (`#scope-canvas`), Logic Analyzer (`#logic-canvas`), and Signal Generator (`#gen-preview-canvas`) to dynamically query CSS custom properties (`getComputedStyle()`), ensuring zero hardcoded color values.
- **DaisyUI Component Replacement**:
  - DSO & Instrument Frames: Replace custom containers with `.card .card-border .card-body`.
  - Telemetry & Measurements: Replace custom metric chips with `.stats .stat` (`.stat-title`, `.stat-value`, `.stat-desc`).
  - Instrument Controls: Replace custom pill buttons with `.join`, `.select .select-sm`, and `.range .range-xs`.
  - Protocol Decoder Traffic Log: Replace custom table markup with `.table .table-zebra .table-pin-rows .table-sm`.
  - REPL & SCPI Terminal: Replace custom output panes with `.mockup-code`.
  - Power & Rail Telemetry: Standardize with `.badge` + `.status .status-success` / `.status-warning`.
  - Flash Storage Meter: Standardize with `.progress .progress-primary`.
  - System Header Bar: Standardize with flat `.navbar` across `/lab` and `/apps`.
  - Settings & Modals: Standardize with native `<dialog class="modal">` and `.modal-box`.
- **Public Website Parity**: Update `web/index.html` and `web/landing.css` to consume DaisyUI tokens and flat card layouts matching the on-device firmware.

### Non-Goals
- **Replacing Canvas 2D with DOM / SVG Elements**: We will NOT replace the waveform canvas with SVG or DOM nodes. High-throughput sampling (up to 500 kS/s and 100k points @ 60 FPS) requires Canvas 2D context for performance.
- **Introducing Node.js / NPM Build Steps**: Firmware HTML/CSS must continue to compile cleanly and serve directly from RP2350 microcontroller flash memory without an external bundler runtime.

## Decisions

### Decision 1: High-Throughput Canvas 2D with Dynamic CSS Custom Property Resolution
- **Choice**: Keep HTML5 Canvas 2D for DSO and Logic Analyzer, but eliminate all hardcoded hex color literals in JavaScript rendering routines.
- **Mechanism**: A theme resolution helper in `firmware/html/app.js` queries `getComputedStyle(document.documentElement)`:
  - Background: `var(--color-base-100)` or `var(--canvas-bg)`
  - Grid reticle divisions: `var(--color-base-300)` or `var(--border-color)`
  - Trace colors: `var(--color-primary)` (CH1/D0-7), `var(--color-secondary)` (CH2/D8-15), `var(--color-accent)` (CH3/D16-23), `var(--color-info)` (CH4/Markers)
- **Rationale**: Retains 60 FPS performance for 100,000+ waveform samples while achieving 100% theme adaptability across all light and dark palettes.
- **Alternatives Considered**:
  - *SVG or HTML DOM points*: Would crash the browser DOM engine at high sample rates.
  - *WebGL shader pipeline*: High complexity, unnecessary memory overhead for 2D waveforms on embedded clients.

### Decision 2: Pure DaisyUI v5 Semantic Color & Elevation Tokens
- **Choice**: Adhere strictly to DaisyUI v5 color semantics (`base-100`, `base-200`, `base-300`, `base-content`, `primary`, `secondary`, `accent`, `neutral`, `info`, `success`, `warning`, `error`).
- **Elevation**:
  - Resting cards: `background-color: var(--color-base-200); border: 1px solid var(--color-base-300); box-shadow: none;`
  - Hovered cards: `transform: translateY(-2px); box-shadow: 0 4px 12px -2px rgb(0 0 0 / 0.15);`
- **Removal**: Delete `--card-neumorphic-shadow`, `--card-rim-highlight`, `linear-gradient` drafting grid, and SVG fractal noise overlays.
- **Rationale**: Eliminates the "dirty/washed-out" appearance in light mode (`emerald`, `caramellatte`) and aligns with the clean, modern Refactoring UI guidelines emphasized in DaisyUI documentation.

### Decision 3: Standardize UI Primitives onto DaisyUI Component Classes
- **Choice**: Replace bespoke custom CSS classes across `/lab`, `/apps`, and `web/` with standard DaisyUI component classes:
  - Top navigation: `.navbar` (solid `bg-base-100`, `border-b border-base-300`)
  - Telemetry: `.stats .stat` (`.stat-title`, `.stat-value`, `.stat-desc`)
  - Decoder logs: `.table .table-zebra .table-pin-rows .table-sm`
  - Shell / Terminal: `.mockup-code`
  - App Launcher squircle tiles: `.card` with flat `base-200` squircle icon containers and 1px `base-300` borders
  - Progress: `.progress .progress-primary`
- **Rationale**: Dramatically reduces CSS maintenance footprint and ensures accessible, predictable responsive behavior.

## Risks / Trade-offs

- **[Risk] CSS Variable query performance during canvas render loops**: Calling `getComputedStyle()` every frame at 60 FPS could cause layout thrashing.
  - **Mitigation**: Cache resolved canvas palette colors in JavaScript and only re-query `getComputedStyle()` when a theme mutation or `agentla-theme-changed` event is dispatched.
- **[Risk] Existing automated UI tests checking for legacy CSS class names**: Tests in `test/test_webui_assets.py` might assert presence of legacy tokens or card structures.
  - **Mitigation**: Update tests to assert pure DaisyUI classes (`navbar`, `stats`, `table-zebra`, `mockup-code`) and assert the complete absence of neumorphic shadow tokens.
- **[Risk] Firmware flash budget limits on RP2350**: Including full unpurged CSS files could exceed microcontroller flash space.
  - **Mitigation**: The compiled stylesheet is compressed/embedded directly; purging redundant bespoke CSS while adding concise DaisyUI utility definitions actually reduces total CSS footprint.

## Migration Plan

1. **Stylesheet Cleanup**:
   - Strip neumorphic variables, millimeter grid background, and noise pseudo-elements from `firmware/html/style.css`.
   - Add/verify DaisyUI component definitions (`.navbar`, `.stats`, `.stat`, `.table-zebra`, `.mockup-code`, `.card-border`, `.join`, `.range`).
2. **HTML Markup Standardization**:
   - Update `firmware/html/index.html` to use DaisyUI component classes.
   - Update `firmware/html/apps.html` to use DaisyUI component classes.
   - Update `web/index.html` and `web/landing.css`.
3. **Canvas Token Binding**:
   - Update `firmware/html/app.js` with cached palette lookup and theme change listener.
4. **Verification**:
   - Run Python asset tests (`pytest test/test_webui_assets.py test/test_multi_app_store.py`).
   - Compile firmware binary (`make -C build-work -j4 AgentLA`).
