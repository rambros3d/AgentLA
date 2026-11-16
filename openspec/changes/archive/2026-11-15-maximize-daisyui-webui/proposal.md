## Why

The AgentLA WebUI currently bundles the complete official daisyUI 5 CSS distribution (`webui/daisyui.css`), but the application stylesheet (`webui/style.css`, 8,334 lines / 200 KB) contains nearly 2,000 lines of redundant in-tree re-implementations of daisyUI components (navbar, cards, buttons, badges, modals, tabs, steps, timeline, chat, diff, stats) and manually polyfilled layout utilities. Furthermore, modal dialogs, dropdowns, telemetry readouts, and form fields are split across legacy custom CSS containers (`.modal-backdrop`, `.profile-dropdown-wrapper`, `.modal-input`) and modern daisyUI components. This causes CSS cascade collisions, excessive flash footprint on the RP2350 microcontroller LittleFS partition, and fragile UI maintenance.

Maximizing daisyUI 5 component usage establishes a single canonical component vocabulary, reduces maintenance burden, and shrinks the uncompressed and gzipped footprint of the embedded web interface.

## What Changes

- **Purge Redundant Component Re-implementations in style.css**: Remove ~1,900 lines of hand-crafted CSS replicating daisyUI primitives (lines 6288-7682) and utility polyfills (lines 7858-8334) that are already provided by `daisyui.css`.
- **Standardize Modals to HTML5 `<dialog class="modal">`**: Refactor legacy div-based modal backdrops (`#modal-settings`, `#modal-url-install`, `#modal-repo-config` in `index.html`, and `#modal-pin-config`, `#modal-settings`, `#modal-add-theme`, `#modal-save-profile`, `#modal-profile-compat`, `#modal-agent-guide` in `lab.html`) to native daisyUI `<dialog class="modal">` using `.modal-box`, `.modal-action`, and native `.showModal()` / `.close()`.
- **Standardize Dropdowns and Split Buttons**: Refactor custom top-bar Hardware Profile popovers (`#profile-dropdown-wrapper`) to daisyUI `.dropdown .dropdown-end` and `.dropdown-content menu`, eliminating custom JavaScript open/close display toggles.
- **Adopt daisyUI Form Primitives**: Replace `.modal-input`, `.modal-select`, and custom controls across all dialogs and workbench panels with daisyUI `.input .input-sm`, `.select .select-sm`, `.range`, `.toggle`, `.checkbox`, and `.fieldset`.
- **Standardize Navigation and Telemetry Surfaces**: Refactor Workbench navigation tabs to daisyUI `.tabs .tabs-box`, hardware telemetry HUD meters to daisyUI `.stats .stat`, and Bento / station tiles to daisyUI `.card .card-border .bg-base-200`.
- **Harmonize Theme Variable Mappings**: Direct legacy CSS variable references (`--bg-surface`, `--text-primary`, `--border-color`) to canonical daisyUI 5 OKLCH theme tokens (`--color-base-100`, `--color-base-200`, `--color-base-content`, `--color-primary`, `--color-error`).
- **Preserve Critical Design Invariants**: Ensure the floating dock remains anchored to the bottom edge (`bottom: 0`), app viewports remain full-bleed (`height: calc(100vh - 48px)`), launcher horizontally centered (`margin-inline: auto`), and strictly zero dot indicator pips or emojis throughout.

## Capabilities

### New Capabilities
- `daisyui-component-maximization`: Standardize WebUI surfaces across Launcher, Workbench, file manager, and bridge injector to native daisyUI 5 component primitives, dialogs, dropdowns, form controls, and stats, while purging custom CSS duplication in `style.css`.

### Modified Capabilities
- `webui-control-surface`: Update modal dialog specifications (Agent Guide, pin configuration, settings) and top bar controls to require standard daisyUI `<dialog class="modal">` and `.dropdown` contracts instead of legacy custom backdrop containers.

## Impact

- **Affected Files**:
  - `webui/style.css`: Significant deletion of redundant component rules (~2,000 lines removed).
  - `webui/index.html`: Modal and dropdown structure refactoring.
  - `webui/lab.html`: Modal dialogs, tabs, and control refactoring.
  - `webui/agentla-bridge.js`: Top bar profile dropdown and modal alignment.
  - `webui/app.js`: Simplification of modal opening/closing logic from DOM manipulation to native dialog methods.
- **Microcontroller Footprint**: Smaller `style.css` reduces LittleFS storage requirements and speeds up WebUI updates.
- **APIs and Runtime**: Zero breaking changes to REST, MCP, or WebSocket endpoints. Purely frontend presentation and maintenance optimization.
