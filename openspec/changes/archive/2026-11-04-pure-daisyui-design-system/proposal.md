## Why

AgentLA currently contains lingering neumorphic styling artifacts: heavy dual-source drop shadows, white inset specular rim highlights (`--card-neumorphic-shadow`, `--card-rim-highlight`), a background CAD millimeter drafting grid, and fractal noise textures. These artifacts clash with light mode palettes (`emerald`, `caramellatte`), wash out borders, and contradict DaisyUI v5's design language. Additionally, oscilloscope and logic analyzer canvases still rely on hardcoded dark colors rather than dynamically inheriting theme tokens from CSS, and the public landing website diverges from the embedded interface.

This change delivers a complete, cohesive revamp to pure DaisyUI across the entire ecosystem: the core workbench (`/lab`), the OS App Launcher (`/apps`), and the public marketing landing website (`web/`).

## What Changes

- **Eliminate All Neumorphism & Bevels**:
  - Remove `--card-neumorphic-shadow` and `--card-rim-highlight`.
  - Remove all `inset 0 1px 0 rgba(255,255,255,...)` highlights from cards, squircle icons, and buttons.
  - Remove the background millimeter CAD drafting grid (`linear-gradient` on body) and the fractal noise texture overlay (`body::before`).
- **Pure DaisyUI Semantic Surfaces & Elevation**:
  - Resting state: Pure flat `var(--color-base-200)` surfaces with crisp 1px borders (`var(--color-base-300)` / `var(--border-color)`).
  - Hover state: Gentle lift (`translateY(-2px)`) with subtle ambient shadow (`box-shadow: 0 4px 12px -2px rgb(0 0 0 / 0.1)`).
- **Dynamic Waveform Canvas Token Binding**:
  - Oscilloscope and logic analyzer HTML5 canvases dynamically resolve colors from computed CSS custom properties rather than hardcoded hex values (`#0d1117`).
  - Canvas background resolves to `var(--color-base-100)` or `var(--canvas-bg)`.
  - Grid divisions resolve to `var(--color-base-300)` / `var(--border-color)`.
  - Channel traces resolve to DaisyUI semantic accents (`--color-primary`, `--color-secondary`, `--color-accent`, `--color-info`, `--color-success`, `--color-warning`).
- **Standard Flat DaisyUI Navbar**:
  - Top application bar on both `/lab` and `/apps` transitions to a flat DaisyUI `.navbar` with solid `base-100` / `base-200` background and a 1px `base-300` bottom border.
  - Eliminates frosted glass blur (`backdrop-filter: blur()`).
- **DaisyUI Component Class Alignment**:
  - Standardize buttons (`.btn`, `.btn-sm`, `.btn-primary`, `.btn-outline`, `.btn-ghost`).
  - Standardize badges (`.badge`, `.badge-sm`, `.badge-outline`).
  - Standardize cards (`.card`, `.card-body`, `.card-border`).
  - Standardize toggles (`.toggle`, `.toggle-primary`).
- **OS Squircle App Grid with DaisyUI Flat Tokens**:
  - Retains the tactile squircle icon + label layout in `/apps`, but icons are flat `base-200` with 1px `base-300` borders and clean hover elevation.
- **Landing Website Revamp (`web/`)**:
  - Update `web/index.html` and `web/landing.css` to adopt pure DaisyUI tokens, flat cards, and matching header/footer styling.

## Capabilities

### Modified Capabilities
- `daisyui-bento-theme-system`: Remove all neumorphic tokens, drafting grids, and noise filters; establish pure DaisyUI flat card and elevation tokens.
- `bench-theme-engine`: Dynamic canvas token binding, flat DaisyUI navbar, and standard DaisyUI component classes.
- `analog-oscilloscope`: Bind canvas rendering loops and grid drawing to live CSS custom properties.
- `unified-os-app-launcher`: Apply flat DaisyUI surfaces, 1px borders, and pure DaisyUI navbar to the App Launcher.
- `hosted-marketing-splash`: Reskin the public landing website with matching pure DaisyUI design tokens and flat card layouts.

## Impact

- `firmware/html/style.css`: Clean overhaul eliminating neumorphism, background grid, and noise; adopting DaisyUI component styles (`btn`, `card`, `badge`, `toggle`, `navbar`).
- `firmware/html/index.html`: Update markup classes to DaisyUI conventions and flat navbar.
- `firmware/html/apps.html`: Update launcher markup and styles to flat DaisyUI tokens.
- `firmware/html/app.js`: Implement dynamic CSS token extraction for canvas rendering.
- `web/index.html` & `web/landing.css`: Update public portal to pure DaisyUI styling.
- `tests/test_webui_assets.py`: Update asset tests to verify DaisyUI classes and zero neumorphic tokens.
