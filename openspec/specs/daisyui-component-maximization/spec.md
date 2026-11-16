# daisyui-component-maximization Specification

## Purpose
TBD - created by archiving change maximize-daisyui-webui. Update Purpose after archive.
## Requirements
### Requirement: Native HTML5 Dialog Modals
The WebUI SHALL implement all modal dialogs across `index.html` and `lab.html` using native HTML5 `<dialog class="modal">` elements containing `.modal-box` and `.modal-action` structures, controlled through native JavaScript `.showModal()` and `.close()` methods.

#### Scenario: Opening settings modal
- **WHEN** user clicks the settings button
- **THEN** `#modal-settings` opens using native `.showModal()` with standard daisyUI backdrop blur and centered positioning

#### Scenario: Dismissing modal with escape key
- **WHEN** user presses the Escape key while a modal is visible
- **THEN** the modal closes natively without requiring custom keydown event listeners

#### Scenario: Opening pin configuration dialog
- **WHEN** user clicks a pin row in the workbench pin matrix
- **THEN** `#modal-pin-config` opens via `.showModal()` presenting configuration fields inside `.modal-box`

### Requirement: Top-Bar Dropdown Menus and Split Buttons
The WebUI top bar SHALL implement profile and action dropdowns using daisyUI `.dropdown` and `.dropdown-content menu` containers, combined with `.join` and `.join-item` for split buttons.

#### Scenario: Interacting with Hardware Profile dropdown
- **WHEN** user clicks the Hardware Profile split trigger button (`#btn-profile-toggle`)
- **THEN** the profile menu displays via daisyUI dropdown positioning without manual `style.display` JavaScript overrides

#### Scenario: Asserting Emergency Safe State
- **WHEN** user clicks `#btn-emergency-safe` in the top bar `.join` group
- **THEN** the action executes the emergency safe state command while displaying error styling (`btn-error text-error-content`)

### Requirement: Standard Form Controls and Fieldsets
The WebUI dialogs, settings panels, and workbench instrument controls SHALL use daisyUI form primitives: `.input .input-sm`, `.select .select-sm`, `.range`, `.toggle`, `.checkbox`, and `.fieldset` with `.fieldset-legend`.

#### Scenario: Displaying numeric and text inputs in dialogs
- **WHEN** a modal or panel renders text or numeric configuration fields
- **THEN** inputs carry `class="input input-sm input-bordered"` and adapt to the active theme colors

#### Scenario: Displaying dropdown selectors
- **WHEN** selecting trigger sources, profile slots, or baud rates
- **THEN** selectors carry `class="select select-sm select-bordered"`

### Requirement: Tab Navigation and Telemetry Stats
The WebUI Workbench and system HUD SHALL use daisyUI `.tabs .tabs-box` for navigation switching and `.stats .stat` for telemetry readouts.

#### Scenario: Switching workbench instrument tabs
- **WHEN** user clicks an instrument tab (Logic Analyzer, Oscilloscope, Adapters, Terminal)
- **THEN** the tab bar updates active status using `.tab .tab-active` within `.tabs .tabs-box`

#### Scenario: Displaying power and voltage telemetry HUD
- **WHEN** telemetry events arrive from `/api/v1/system/telemetry`
- **THEN** voltage and current readings display within daisyUI `.stat` blocks with `.stat-title`, `.stat-value`, and `.stat-desc`

### Requirement: Deduplication and Legacy CSS Cleanup
The WebUI stylesheet `webui/style.css` SHALL rely on `webui/daisyui.css` for all daisyUI component classes and layout primitives, deleting redundant in-tree re-implementations.

#### Scenario: Verifying style.css clean footprint
- **WHEN** inspecting `webui/style.css`
- **THEN** manual duplicate definitions of `.navbar`, `.card`, `.btn`, `.badge`, `.modal`, `.alert`, `.tabs`, `.join`, `.toggle`, `.steps`, `.timeline`, `.chat`, `.diff`, `.rating`, `.stats`, and `.collapse` are purged
- **AND** the dock remains anchored at `bottom: 0`, viewports maintain full-bleed geometry, and strictly zero dot indicators or emojis are rendered

