## MODIFIED Requirements

### Requirement: Deep Dark and High-Contrast Light Theme Switching
The WebUI SHALL provide a theme engine supporting dark themes (such as "Forest" OKLCH) and high-contrast light themes (such as "Emerald" and "CaramelLatte" OKLCH) using pure DaisyUI v5 semantic tokens, enforcing flat resting surface elevation with 1px borders rather than dual-source neumorphic drop shadows or specular rim highlights. Compiled `themes.css` SHALL structure `:root` fallback variables such that specific theme rules (`[data-theme="emerald"]`, `[data-theme="caramellatte"]`, `[data-theme-scheme="light"]`) take precedence and override default `:root` variables without cascade collision. Both `index.html` and `apps.html` SHALL specify `data-theme="forest"` as their default static HTML markup to guarantee consistent rendering before client scripts execute.

#### Scenario: Default deep dark theme application
- **WHEN** the WebUI loads without existing user preference
- **THEN** the system applies the dark palette with flat card surfaces (`oklch(24% 0 0)`) over dark page canvas (`oklch(18% 0 0)`), high-contrast trace colors, and 1px borders (`oklch(28% 0 0)`) without dual-layer neumorphic shadows.

#### Scenario: Toggle to light theme
- **WHEN** the user taps the theme toggle button in the top navigation bar or selects Light Mode in Settings
- **THEN** the root CSS variables instantly switch to clean high-contrast light colors with flat white surfaces (`oklch(100% 0 0)`) over off-white canvas (`oklch(93% 0 0)`) and dark text without requiring page reload.

#### Scenario: Theme persistence across reloads
- **WHEN** the user switches themes and refreshes or revisits the page
- **THEN** the selected theme mode (`agentla-theme-mode`) and active theme name (`agentla-theme`) are restored immediately from `localStorage` before page rendering.

### Requirement: Dynamic Waveform Canvas Theme Token Binding
The WebUI JavaScript engine SHALL dynamically extract color tokens from the active DaisyUI theme via CSS custom properties and apply them to HTML5 Canvas 2D render loops for the Oscilloscope, Logic Analyzer, and Signal Generator without hardcoded color literals.

#### Scenario: Switching theme updates canvas palette
- **WHEN** the active theme is changed from dark to light
- **THEN** `#scope-canvas` and `#logic-canvas` redraw their background, reticle grid, and waveform channel traces using dynamically queried theme tokens (`--color-base-100`, `--color-base-300`, `--color-primary`, `--color-secondary`, `--color-accent`)
