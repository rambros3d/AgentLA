## Context

AgentLA's WebUI is a browser-based hardware instrumentation interface running directly off the Raspberry Pi RP2350 microcontroller (serving via lwIP HTTPD). Recent experiments attempting faux-neumorphism (soft extruded shadows, inset highlights) proved ineffective across both dark and light palettes due to lack of contrast, luminance clipping in light mode, and visual clutter. In contrast, DaisyUI v5's design tokens provide crisp border styling (`--border: 1px/2px`), authentic elevation, tactile physical depth (`--depth: 1`), and grain texture (`--noise: 1`).

Additionally, several user-facing bugs have impaired hardware workflows:
1. **Scope Channel Visibility**: Probing pins GP10–GP19 failed to render on the logic analyzer canvas due to rigid channel offset calculations (`p - basePin` with `basePin=20`).
2. **Interactive Pin Selector**: Color swatches in `#modal-pin-config` and status badges in `#scope-pins-bar` suffered from CSS selector drift (`.color-swatch-btn` vs `.swatch-item` and `.configured-digital` vs `.active-digital`).
3. **Adapter Terminal Freezing**: Bus master transactions (UART, I2C, SPI, CAN, Modbus) logged to a secondary history table but bypassed the primary adapter terminal window (`appendAdapterLog()`).
4. **Pin Matrix Buttons**: The instant GPIO action buttons (`.btn-pin-act` for High, Low, Pull-Up, Pull-Down, High-Z) lacked CSS styling.
5. **Persistence & Architecture Coupling**: Theme and settings persistence was previously entangled with browser `localStorage` or considered for MCP.

## Goals / Non-Goals

**Goals:**
- Provide a modular DaisyUI v5 theme architecture with dedicated theme files in `themes/`:
  - `themes/emerald.css` (Default Light)
  - `themes/forest.css` (Default Dark)
  - `themes/caramellatte.css` (Warm Cream & Espresso Light)
- Implement a Python-based theme compilation pipeline (`tools/compile_themes.py`) that transpiles `@plugin "daisyui/theme"` syntax into `firmware/html/themes.css` and generates `firmware/html/themes.json` metadata for the WebUI.
- Provide a top navigation bar Tri-State Theme Switcher cycling through `System` &rarr; `Light` &rarr; `Dark` &rarr; `System` with Solar BoldDuotone icons (`sun`, `moon`, and split `sun-fog`).
- Update Settings Modal with Default Light and Default Dark theme selectors, plus an "Add Theme" modal where users can paste raw DaisyUI `@plugin` CSS, preview live, and persist to flash.
- Remove obsolete hue sliders and purge hardcoded `var(--accent-cyan)` tokens in favor of `var(--color-primary)` and semantic theme tokens.
- Establish a clean Hybrid Storage architecture:
  - Hardware identity, active theme, default themes, and custom pasted themes persist to LittleFS Flash (`/settings.json`, `/themes/*.css`) via firmware REST endpoints `/api/v1/settings` and `/api/v1/themes`.
  - Zero UI settings endpoints in `/mcp`, keeping MCP strictly dedicated to AI agent hardware tools.
  - Eliminate cold-start FOUC using lwIP SSI (`<html data-theme="<!--#THEME-->">`).
  - Retain high-frequency UI states (e.g. MSO split pane dragging) in browser `localStorage` to preserve RP2350 flash endurance.
- Fix Scope and Adapter interactive bugs:
  - Dynamic desktop sidebar rail tabs: mirror the mobile bottom nav's dynamic (icon - icon/text) pattern, where inactive tabs show only their Solar icon and the active tab expands to show icon + text label with smooth CSS transitions.
  - Virtual pin mapping for GP10–GP29 to logic analyzer lanes D0–D7.
  - Unify CSS classes for `.swatch-item` and `.active-digital` / `.active-analog`.
  - Stream all adapter transactions to protocol log terminals via `appendAdapterLog()`.
  - Style Pin Matrix `.btn-pin-act` buttons using DaisyUI button group aesthetics.

**Non-Goals:**
- Modifying underlying PIO capture programs or hardware DMA streaming engines.
- Adding arbitrary multi-channel analog oscilloscopes beyond RP2350's hardware ADC pins (GP26–GP29).
- Modifying AI MCP protocol schema or hardware command execution in `/mcp`.

## Decisions

### 1. Dedicated `themes/` Directory & DaisyUI Plugin Syntax
- **Decision**: Store themes as standalone `.css` files in `themes/` using raw DaisyUI v5 plugin format:
  ```css
  @plugin "daisyui/theme" {
    name: "emerald";
    default: true;
    prefersdark: false;
    color-scheme: "light";
    --color-base-100: oklch(100% 0 0);
    ...
    --depth: 1;
    --noise: 1;
  }
  ```
- **Rationale**: Developers and users can copy-paste theme definitions directly from DaisyUI theme generators or official documentation without manual JSON translation.
- **Alternatives Considered**: Storing themes as static JSON objects in `app.js`. Rejected because it loses CSS tooling support, prevents raw CSS pasting, and complicates user customization.

### 2. Theme Compilation Pipeline (`tools/compile_themes.py`)
- **Decision**: A lightweight Python compiler parses all `@plugin "daisyui/theme"` blocks in `themes/*.css`, extracting metadata (name, color-scheme, prefersdark) and generating:
  1. `firmware/html/themes.css`: CSS with `[data-theme="<name>"] { ... }` attribute selectors.
  2. `firmware/html/themes.json`: Array of available theme metadata for UI dropdowns.
  Integrated directly into `CMakeLists.txt` and `tools/gen_board_assets.py`.
- **Rationale**: Keeps embedded runtime footprint minimal, eliminates clientside CSS parsing overhead on RP2350, and automatically keeps firmware assets in sync during build.

### 3. Top Navigation Bar Tri-State Switcher
- **Decision**: Replace binary toggle with a 3-way toggle button cycling:
  `System (auto OS detection)` &rarr; `Light (default light theme)` &rarr; `Dark (default dark theme)` &rarr; `System`.
  Icon switches between Solar BoldDuotone `sun-fog` (split sun/moon for System), `sun` (Light), and `moon` (Dark).
- **Rationale**: Conforms to modern OS/desktop expectations while allowing lab engineers to quickly force high-contrast light mode under bright lights or dark mode in dim testing setups.

### 4. Hybrid Flash & LocalStorage Model
- **Decision**:
  - LittleFS Flash (`/settings.json`): Persists `theme_mode` (`system`, `light`, `dark`), `default_light_theme`, `default_dark_theme`, and active profile.
  - LittleFS Flash (`/themes/<custom>.css`): Persists user-created custom themes.
  - Browser `localStorage`: Persists `agentla_mso_split` (drag position of oscilloscope vs logic analyzer) and interactive console command history.
- **Rationale**: Flash writes are limited to intentional configuration saves (tens of times per month). High-frequency drag operations at 60Hz are isolated to RAM/localStorage to protect flash endurance.

### 5. Decoupled REST Endpoints vs MCP Tools
- **Decision**: Introduce `/api/v1/settings` and `/api/v1/themes` in `firmware/src/rest_api.c`. Do not add any UI settings to `/mcp`.
- **Rationale**: MCP is an agentic tool execution transport for AI coding assistants. Mixing UI presentation state into MCP pollutes agent tool definitions. The firmware's native REST API is the proper home for WebUI configuration.

### 6. Dynamic Virtual Channel Multiplexing for Logic Analyzer
- **Decision**: In `app.js`, logic analyzer channel rendering will not assume `pin - 20`. Instead, active digital pins from `state.configuredPins` are sorted and mapped to digital lanes `D0` through `D7`:
  `pinToLaneMap.set(pin, laneIndex)`.
- **Rationale**: Probe pins GP10 through GP25 can now be assigned to any logic analyzer channel without being clipped or dropped.

## Risks / Trade-offs

- **[Risk] LittleFS flash write exhaustion from repeated theme edits** &rarr; *Mitigation*: Flash commits are strictly debounced and triggered only on explicit "Save" button clicks in the modal. Splitter resizing and UI state stay in `localStorage`.
- **[Risk] CSS bloat from multiple bundled themes on RP2350 flash** &rarr; *Mitigation*: Each DaisyUI theme is roughly 800 bytes; three themes total under 3 KB uncompressed, and `gzip -9` reduces them to < 800 bytes embedded in flash.
- **[Risk] FOUC when booting WebUI with custom dark theme** &rarr; *Mitigation*: lwIP HTTPD Server-Side Include (`<!--#THEME-->`) populates `<html data-theme="...">` during initial HTML stream before DOM parsing.

## Migration Plan

1. Create `themes/` with `emerald.css`, `forest.css`, `caramellatte.css`.
2. Implement and test `tools/compile_themes.py`. Hook into CMake build.
3. Implement `/api/v1/settings` and `/api/v1/themes` in `firmware/src/rest_api.c` and test with `curl` / LittleFS.
4. Update `firmware/src/network_stack.c` with `<!--#THEME-->` SSI tag.
5. Update `firmware/html/index.html`, `firmware/html/style.css`, and `firmware/html/app.js` with:
   - Tri-State Theme Switcher and Solar icons.
   - Settings Modal dropdowns and "Add Theme" modal.
   - Removal of hue slider and purge of `var(--accent-cyan)`.
   - Virtual channel mapping in logic analyzer.
   - Adapter terminal logging in `handleBusTransmitWithAutoSniff()`.
   - Pin Matrix `.btn-pin-act` button styles.
6. Verify via mock bridge and clean firmware build (`make -C build-work -j4 AgentLA`).
