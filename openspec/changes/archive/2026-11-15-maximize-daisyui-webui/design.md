## Context

The AgentLA WebUI provides high-speed logic analyzer, oscilloscope, protocol decoder, and app store surfaces on the RP2350 microcontroller. Assets are stored in a dedicated 512 KB LittleFS system partition and streamed over HTTP.

Currently, `webui/daisyui.css` bundles the official daisyUI 5.7.50 release. However, `webui/style.css` (8,334 lines, 200 KB) contains nearly 2,000 lines of manual re-definitions of daisyUI components (navbar, card, button, badge, modal, tabs, alert, toggle, join, kbd, steps, timeline, chat, diff, stats) and hand-crafted utility classes. Simultaneously, interactive dialogs and popovers across `index.html` and `lab.html` are split between legacy div-based backdrops (`.modal-backdrop` with manual `style.display` manipulation) and standard HTML5 `<dialog class="modal">` elements.

## Goals / Non-Goals

**Goals:**
- Maximize standard daisyUI 5 component primitives across all WebUI pages (`index.html`, `lab.html`, `files.html`, `terminal.html`, `agentla-bridge.js`).
- Eliminate roughly 1,900 lines of redundant duplicate component declarations and utility polyfills from `style.css`.
- Unify all modal dialogs onto native HTML5 `<dialog class="modal">` with `.modal-box`, `.modal-action`, and native `.showModal()` / `.close()`.
- Unify top-bar dropdowns and split-buttons onto daisyUI `.dropdown` and `.join`.
- Replace legacy form classes (`.modal-input`, `.modal-select`) with daisyUI `.input-sm`, `.select-sm`, `.range`, and `.fieldset`.
- Preserve all strict design rules: dock anchored to bottom edge (`bottom: 0`), full-bleed app viewport (`height: calc(100vh - 48px)`), launcher centered horizontally (`margin-inline: auto`), and strictly zero dot indicator pips or emojis.

**Non-Goals:**
- Modifying backend Rust firmware, picoserve routing, or LittleFS partition allocation.
- Altering REST API schemas or MCP JSON-RPC protocol contracts.
- Introducing a Node.js / npm runtime build step into the firmware build pipeline; assets remain pre-bundled static web distribution files.

## Decisions

### Decision 1: HTML5 `<dialog class="modal">` for All Dialogs
- **Choice**: Convert all legacy custom modal backdrops (`#modal-settings`, `#modal-pin-config`, `#modal-add-theme`, `#modal-save-profile`, `#modal-profile-compat`, `#modal-agent-guide`, `#modal-url-install`, `#modal-repo-config`) to standard `<dialog class="modal">`.
- **Rationale**: DaisyUI 5 natively styles `<dialog class="modal">`, providing built-in backdrop blur, responsive centering, automatic Escape-key dismissal, and clean programmatic control via element methods `.showModal()` and `.close()`.
- **Alternatives Considered**: Keeping `.modal-backdrop` div wrappers was rejected because it requires maintaining 150+ lines of custom position/flex CSS and custom keyboard event handling in JavaScript.

### Decision 2: Pure CSS daisyUI Dropdown for Hardware Profiles
- **Choice**: Use daisyUI `.dropdown .dropdown-end` with `.dropdown-content menu bg-base-200 rounded-box border border-base-300` for top-bar profile selection.
- **Rationale**: DaisyUI dropdowns operate via focus/blur and popover mechanics without requiring manual JavaScript event listeners to toggle `style.display = 'block' | 'none'`, preventing state desynchronization bugs.
- **Alternatives Considered**: Keeping `#profile-dropdown-menu` with custom JavaScript click outside handlers was rejected due to unnecessary complexity and maintenance friction.

### Decision 3: Selective Purge of style.css Lines 6288-7682 and 7858-8334
- **Choice**: Delete duplicate `.navbar`, `.card`, `.btn`, `.modal`, `.tabs`, `.join`, `.toggle`, `.badge`, `.steps`, `.timeline`, `.chat`, `.diff`, `.rating`, `.stats` blocks from `style.css`, relying entirely on `daisyui.css`. Keep only custom application-specific rules (canvas waveform renderers, pin matrix table borders, dock glass effect).
- **Rationale**: `daisyui.css` already contains all these rules. Deleting the duplicates reduces `style.css` size by ~25% and eliminates cascade fighting.
- **Alternatives Considered**: Re-running a custom Tailwind build was rejected because `daisyui.css` is already a working standalone asset in the repository.

### Decision 4: Fieldset and Input Standardization
- **Choice**: Adopt daisyUI 5 `.fieldset`, `.fieldset-legend`, `.input .input-sm`, and `.select .select-sm` for modal forms and settings panels.
- **Rationale**: DaisyUI 5 introduced native fieldset wrappers that integrate labels, error hints, and input boundaries cleanly into theme OKLCH color spaces.

## Risks / Trade-offs

- **[Risk]** Existing JavaScript click listeners expecting `style.display === 'none'` on modals might fail.
  - **Mitigation**: Update all modal openers and closers in `app.js` and `index.html` to call `.showModal()` and `.close()`, or check `.open` attribute.
- **[Risk]** Removing utility classes from `style.css` could break elements relying on utility classes if not present in `daisyui.css`.
  - **Mitigation**: Verify which utility classes are present in `daisyui.css` vs custom in `style.css`. Retain essential layout utilities (`.flex`, `.items-center`, `.grid`) if not already present in `daisyui.css`, while deleting all component duplicates.
- **[Risk]** Visual regression on floating dock or pin matrix.
  - **Mitigation**: Inspect and verify against project rules (dock `bottom: 0`, full-bleed viewports, no dot indicator pips).

## Migration Plan

1. **Audit & Deduplicate CSS**: Remove component duplication sections in `style.css`.
2. **Refactor HTML Dialogs**: Convert modals in `index.html` and `lab.html` to `<dialog class="modal">`.
3. **Refactor Top-Bar Dropdown**: Convert Hardware Profile popovers in `index.html`, `lab.html`, and `agentla-bridge.js` to daisyUI `.dropdown`.
4. **Update JS Handlers**: Update modal open/close calls in `app.js` and `index.html` to use `.showModal()` and `.close()`.
5. **Verify Build & Headroom**: Run `tools/pack_webui.py` to confirm the compressed image fits comfortably within LittleFS system partition limits.

## Open Questions

- None. All target components are fully supported in bundled `daisyui.css` v5.7.50.
