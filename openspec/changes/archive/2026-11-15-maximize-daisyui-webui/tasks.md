## 1. Clean & Deduplicate Stylesheets

- [x] 1.1 Remove redundant daisyUI component re-definitions in `webui/style.css` (lines 6288-7682: `.navbar`, `.card`, `.btn`, `.badge`, `.alert`, `.modal`, `.tabs`, `.join`, `.toggle`, `.table`, `.kbd`, `Steps`, `Timeline`, `Chat`, `Diff`, `Rating`, `Stats`)
- [x] 1.2 Retain custom instrument canvas, pin matrix, and floating dock rules while pruning redundant utility overrides
- [x] 1.3 Map legacy CSS variables (`--bg-surface`, `--text-primary`, `--border-color`) to canonical daisyUI 5 OKLCH tokens (`--color-base-100`, `--color-base-200`, `--color-base-content`, `--color-primary`)

## 2. Standardize Modals to Native HTML5 Dialogs

- [x] 2.1 Refactor Launcher modals in `webui/index.html` (`#modal-settings`, `#modal-url-install`, `#modal-repo-config`) from `.modal-backdrop` divs to `<dialog class="modal">` containing `.modal-box` and `.modal-action`
- [x] 2.2 Refactor Workbench modals in `webui/lab.html` (`#modal-pin-config`, `#modal-settings`, `#modal-add-theme`, `#modal-save-profile`, `#modal-profile-compat`, `#modal-agent-guide`) to `<dialog class="modal">`
- [x] 2.3 Update modal trigger and dismissal functions in `webui/app.js` and `webui/index.html` to use `.showModal()` and `.close()` instead of manual `style.display` toggling

## 3. Standardize Top Bar Dropdowns & Split Buttons

- [x] 3.1 Refactor Hardware Profile split-button in `webui/index.html` and `webui/lab.html` to use daisyUI `.dropdown .dropdown-end` with `.dropdown-content menu`
- [x] 3.2 Update dynamic top bar injection in `webui/agentla-bridge.js` to use the standard daisyUI `.dropdown` structure
- [x] 3.3 Simplify profile selection logic in `webui/app.js` and `webui/agentla-bridge.js` to rely on native CSS focus/blur behavior without manual display overrides

## 4. Standardize Form Controls & Navigation Surfaces

- [x] 4.1 Replace `.modal-input` and `.modal-select` with daisyUI `.input .input-sm .input-bordered` and `.select .select-sm .select-bordered` across all dialog forms
- [x] 4.2 Standardize Workbench instrument tab bar in `webui/lab.html` to daisyUI `.tabs .tabs-box`
- [x] 4.3 Format top-bar and HUD telemetry readouts with daisyUI `.stats`, `.stat`, and `.badge` primitives

## 5. Verification & Packaging

- [x] 5.1 Run `tools/pack_webui.py` to confirm that the packaged `webui.img` fits within the 512 KB LittleFS partition with >= 35% headroom
- [x] 5.2 Validate design invariants: floating dock remains anchored at `bottom: 0`, viewports are full-bleed (`calc(100vh - 48px)`), launcher remains centered horizontally, and strictly zero dot indicator pips or emojis are present
