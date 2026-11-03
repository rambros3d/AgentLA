## Why

AgentLA's WebUI currently suffers from several visual, functional, and architectural issues:
1. The attempted "neumorphic" styling fails visually because flat `#ffffff` cannot project positive highlights without high contrast borders and authentic elevation, causing UI elements to look muddy and low-contrast. DaisyUI v5's native design system—with modular tokens, physical `--depth: 1`, `--noise: 1`, and clean semantic borders—provides superior visual depth and ergonomics for test and measurement instruments.
2. Key interactive workflows across the workbench are broken by selector mismatches and unhandled event routing:
   - The Scope pin configuration modal swatch selector and pins bar fail to highlight active states due to CSS class drift (`.color-swatch-btn` vs `.swatch-item`; `.configured-digital` vs `.active-digital`).
   - Probing pins GP10–GP19 drops digital signals entirely because the logic analyzer rigidly offsets channels from GP20 instead of multiplexing active pins to virtual lanes D0–D7.
   - The Adapter protocol terminals (I2C, SPI, UART, CAN) freeze on placeholder text during transmissions because the send handler bypasses `appendAdapterLog()`.
   - Pin Matrix instant GPIO toggle buttons lack visual styling (`.btn-pin-act`).
3. Theme persistence and settings have been awkwardly conflated with MCP or browser `localStorage`. MCP should remain strictly dedicated to AI agent hardware tools, while board settings and theme definitions should be managed through native firmware REST endpoints (`/api/v1/settings`, `/api/v1/themes`) backed by LittleFS flash, paired with Server-Side Includes (SSI) to eliminate cold-boot Flash of Unstyled Content (FOUC). High-frequency layout states (like MSO splitter dragging) should remain in browser `localStorage` to protect RP2350 flash endurance.

## What Changes

- **DaisyUI Modular Theme Engine**:
  - Establish a dedicated `themes/` directory at the repository root containing `@plugin "daisyui/theme"` files:
    - `themes/emerald.css` (Default Light)
    - `themes/forest.css` (Default Dark)
    - `themes/caramellatte.css` (Warm Cream / Espresso Light)
  - Create a theme compiler build step (`tools/compile_themes.py`) integrated into `CMakeLists.txt` and `tools/gen_board_assets.py` to transpile theme files into `firmware/html/themes.css`.
  - Implement a top navigation bar **Tri-State Theme Switcher** cycling between `System` (auto OS detection), `Light`, and `Dark`, with Solar BoldDuotone icons (`sun`, `moon`, and split `sun-fog`).
  - Upgrade the Settings modal with Default Light and Default Dark theme selectors and an "Add Theme" modal where users can paste custom `@plugin "daisyui/theme"` CSS, preview live, and persist to flash.
  - Remove deprecated hue sliders and hardcoded `var(--accent-cyan)` tokens.
- **Decoupled REST & Hybrid Storage Architecture**:
  - Keep `/mcp` strictly reserved for agentic hardware tools.
  - Implement `/api/v1/settings` (`GET`/`POST`) and `/api/v1/themes` (`GET`/`POST`) in `firmware/src/rest_api.c` backed by LittleFS flash (`/settings.json` and `/themes/*.css`).
  - Inject active theme into HTML via lwIP SSI tag `<!--#THEME-->` (`<html data-theme="<!--#THEME-->">`) to eliminate FOUC.
  - Store high-frequency layout changes (e.g. MSO splitter dragging) in browser `localStorage` to avoid flash wear.
- **Scope & Probe Pin Mapping Fixes**:
  - Unify CSS classes between `app.js` and `style.css` for `.swatch-item` and `.active-digital` / `.active-analog`.
  - Implement Virtual Channel Multiplexing: map user-selected probe pins (GP10–GP29) dynamically to digital logic analyzer lanes D0–D7.
- **Adapter, Sidebar & Pin Matrix Fixes**:
  - Update desktop vertical navigation sidebar tabs to be dynamic like the mobile bottom nav (inactive tabs display icon only; active tab dynamically reveals the label with smooth animation).
  - Connect all bus master transmissions (UART, I2C, SPI, CAN, Modbus) to `appendAdapterLog()` with timestamps and hex/ASCII formatting.
  - Style Pin Matrix `.btn-pin-act` instant GPIO toggler buttons with DaisyUI button groups.

## Capabilities

### Modified Capabilities
- `bench-theme-engine`: Replace faux-neumorphism with modular DaisyUI themes (`--depth: 1`, `--noise: 1`), tri-state topbar switcher, flash REST persistence, and custom theme CSS paste support.
- `probe-pin-configuration`: Fix color swatch and pin status class mismatches, and dynamically map probe pins GP10–GP29 to logic analyzer lanes D0–D7.
- `webui-control-surface`: Route bus master transmissions to adapter log windows, style Pin Matrix instant GPIO buttons, and eliminate residual hardcoded color tokens.

## Impact

- **Firmware**: Updates to `firmware/src/rest_api.c`, `firmware/src/network_stack.c`, `firmware/src/littlefs_driver.c`, and build tools.
- **Web UI**: Updates to `firmware/html/index.html`, `firmware/html/app.js`, `firmware/html/style.css`, and new `firmware/html/themes.css`.
- **Assets / Themes**: New `themes/` directory at repo root with `emerald.css`, `forest.css`, `caramellatte.css`.
- **Build / Packaging**: Theme compiler script `tools/compile_themes.py` hooked into CMake and `tools/gen_board_assets.py`.
- **APIs**: New REST endpoints `/api/v1/settings` and `/api/v1/themes`. No breaking changes to existing MCP tools.
