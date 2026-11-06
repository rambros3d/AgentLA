## MODIFIED Requirements

### Requirement: Deep Dark and High-Contrast Light Theme Switching
The WebUI SHALL provide a theme engine supporting both AgentLA's default Dark theme ("Forest" OKLCH) and a high-contrast Light Theme ("Emerald" OKLCH) for bright laboratory environments, enforcing consistent neumorphic surface elevation (cards elevated above canvas) and dual-layer shadow rendering across both themes. All interactive controls, active navigation tabs, buttons, hardware profile dropdowns, and modals SHALL bind strictly to semantic DaisyUI theme tokens (`--color-base-100`, `--color-base-200`, `--color-base-300`, `--color-primary`, `--border-color`), using Emerald Green (`--color-primary`) as the unified active accent color and prohibiting hardcoded palette colors.

#### Scenario: Default deep dark theme application
- **WHEN** the WebUI loads without existing user preference
- **THEN** the system applies the dark palette with elevated card surfaces (`oklch(24% 0 0)`) over dark page canvas (`oklch(18% 0 0)`), high-contrast trace colors, and dual-layer neumorphic shadows.

#### Scenario: Toggle to light theme
- **WHEN** the user taps the theme toggle button in the top navigation bar
- **THEN** the root CSS variables instantly switch to clean high-contrast light colors with white elevated surfaces (`oklch(100% 0 0)`) over off-white canvas (`oklch(93% 0 0)`) and dark text without requiring page reload.

#### Scenario: Theme persistence across reloads
- **WHEN** the user switches themes and refreshes or revisits the page
- **THEN** the selected theme is restored immediately from `localStorage` before page rendering.

#### Scenario: Elimination of hardcoded palette colors
- **WHEN** opening the hardware profile dropdown or any modal under either Dark or Light theme
- **THEN** surfaces, headers, borders, and input fields adopt the active theme's `--color-base-*` and `--border-color` tokens rather than fixed dark Slate hex values
