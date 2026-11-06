# bench-theme-engine Specification

## Purpose
Provides a modular theme engine supporting dark, light, and warm themes using pure DaisyUI v5 semantic tokens, theme compilation pipelines, settings persistence, and canvas token bindings.

## Requirements

### Requirement: Deep Dark and High-Contrast Light Theme Switching
The WebUI SHALL provide a theme engine supporting modular DaisyUI themes including default Dark theme ("Forest" OKLCH), default Light theme ("Emerald" OKLCH), and warm Light theme ("CaramelLatte" OKLCH), utilizing DaisyUI v5 styling with authentic elevation, borders, `--depth: 1`, and `--noise: 1` rather than faux-neumorphic shadows.

#### Scenario: Default deep dark theme application
- **WHEN** the WebUI loads without an existing user preference
- **THEN** the system applies the default Dark theme ("Forest") with dark canvas base (`oklch(20.84% 0.008 17.911)`), elevated surface containers, vibrant forest green primary accents, `--depth: 1`, and `--noise: 1`.

#### Scenario: Toggle to light theme
- **WHEN** the user taps the theme toggle button in the top navigation bar or selects Light Mode in Settings
- **THEN** the root CSS variables instantly switch to clean high-contrast light colors with flat white surfaces (`oklch(100% 0 0)`) over off-white canvas (`oklch(93% 0 0)`) and dark text without requiring page reload.

#### Scenario: Toggle between light, dark, and system modes
- **WHEN** the user clicks the theme toggle button in the top navigation bar
- **THEN** the active theme mode cycles through `System` → `Light` → `Dark` → `System`
- **AND** the icon updates to match the active state (split `sun-fog` for System, `sun` for Light, `moon` for Dark)
- **AND** the root `data-theme` attribute switches immediately without requiring a page reload.

#### Scenario: Theme persistence across reloads
- **WHEN** the user switches themes or changes default theme preferences in Settings
- **THEN** the preference is saved to LittleFS Flash (`/settings.json`) via `POST /api/v1/settings`
- **AND** on subsequent cold reloads the page loads with `<html data-theme="<!--#THEME-->">` populated via lwIP SSI without Flash of Unstyled Content (FOUC).

#### Scenario: Elimination of hardcoded palette colors
- **WHEN** opening the hardware profile dropdown or any modal under either Dark or Light theme
- **THEN** surfaces, headers, borders, and input fields adopt the active theme's `--color-base-*` and `--border-color` tokens rather than fixed dark Slate hex values.

### Requirement: Dynamic Waveform Canvas Theme Token Binding
The WebUI JavaScript engine SHALL dynamically extract color tokens from the active DaisyUI theme via CSS custom properties and apply them to HTML5 Canvas 2D render loops for the Oscilloscope, Logic Analyzer, and Signal Generator without hardcoded color literals.

#### Scenario: Switching theme updates canvas palette
- **WHEN** the active theme is changed from dark to light
- **THEN** `#scope-canvas` and `#logic-canvas` redraw their background, reticle grid, and waveform channel traces using dynamically queried theme tokens (`--color-base-100`, `--color-base-300`, `--color-primary`, `--color-secondary`, `--color-accent`).

### Requirement: Unified Neumorphic Depth Tokens
The theme engine SHALL expose unified DaisyUI depth, noise, and border tokens across all active themes rather than faux-neumorphic dual-layer box shadows.

#### Scenario: Theme surface rendering in Dark theme
- **WHEN** UI elements render under `[data-theme="forest"]`
- **THEN** surfaces display solid base colors with `--border: 1px`, `--depth: 1`, and `--noise: 1` texture, avoiding low-contrast neumorphic bevels.

#### Scenario: Theme surface rendering in Light theme
- **WHEN** UI elements render under `[data-theme="emerald"]` or `[data-theme="caramellatte"]`
- **THEN** cards and controls display crisp semantic borders (`--border: 1px` or `--border: 2px`), high-contrast text content, `--depth: 1`, and `--noise: 1`.

### Requirement: Dedicated Theme Files and Compiler Pipeline
The project SHALL maintain modular theme definitions in a dedicated `themes/` repository directory using DaisyUI `@plugin "daisyui/theme"` syntax, and provide a build compiler `tools/compile_themes.py` that generates `firmware/html/themes.css` and `firmware/html/themes.json`.

#### Scenario: Compiling theme definitions
- **WHEN** `tools/compile_themes.py` executes during build
- **THEN** it parses `themes/*.css`, validates `@plugin "daisyui/theme"` blocks, extracts theme metadata (name, color-scheme, prefersdark), and writes `themes.css` with `[data-theme="..."]` rules and `themes.json` into `firmware/html/`.

#### Scenario: Bundled theme set
- **WHEN** the WebUI theme engine initializes
- **THEN** it loads metadata for `emerald` (default light), `forest` (default dark), and `caramellatte` (warm light).

### Requirement: Settings Modal Theme Management and Custom Theme Import
The Settings Modal SHALL provide controls to select the default light theme, default dark theme, and paste custom DaisyUI `@plugin "daisyui/theme"` CSS with live preview and flash storage.

#### Scenario: Configuring default light and dark themes
- **WHEN** the user opens `#modal-settings`
- **THEN** dropdown menus display all available themes categorized by light and dark color schemes
- **AND** selecting a new default theme and saving persists the preference to `/api/v1/settings`.

#### Scenario: Adding a custom theme via CSS paste
- **WHEN** the user clicks "Add Theme" in Settings and pastes valid `@plugin "daisyui/theme"` CSS
- **THEN** clicking "Preview" applies the theme live to the current document
- **AND** clicking "Save Theme" uploads the CSS to LittleFS Flash via `POST /api/v1/themes` and registers the theme in the available themes list.

### Requirement: Hybrid Storage Architecture
The system SHALL separate persistent hardware identity and structural settings (Flash) from high-write interactive UI adjustments (`localStorage`).

#### Scenario: Persisting layout adjustments without flash wear
- **WHEN** the user drags the MSO split pane divider between the oscilloscope and logic analyzer
- **THEN** the split ratio is saved immediately to browser `localStorage` under `agentla_mso_split` without triggering flash write cycles.

#### Scenario: Decoupling settings from AI MCP
- **WHEN** the WebUI reads or saves themes and settings
- **THEN** it communicates strictly through REST endpoints `/api/v1/settings` and `/api/v1/themes` without dispatching requests to `/mcp`.
