## MODIFIED Requirements

### Requirement: Deep Dark and High-Contrast Light Theme Switching
The WebUI SHALL provide a theme engine supporting both AgentLA's default Dark theme ("Forest" OKLCH) and a high-contrast Light Theme ("Emerald" OKLCH) for bright laboratory environments, enforcing consistent neumorphic surface elevation (cards elevated above canvas) and dual-layer shadow rendering across both themes.

#### Scenario: Default deep dark theme application
- **WHEN** the WebUI loads without existing user preference
- **THEN** the system applies the dark palette with elevated card surfaces (`oklch(24% 0 0)`) over dark page canvas (`oklch(18% 0 0)`), high-contrast trace colors, and dual-layer neumorphic shadows.

#### Scenario: Toggle to light theme
- **WHEN** the user taps the theme toggle button in the top navigation bar
- **THEN** the root CSS variables instantly switch to clean high-contrast light colors with white elevated surfaces (`oklch(100% 0 0)`) over off-white canvas (`oklch(93% 0 0)`) and dark text without requiring page reload.

#### Scenario: Theme persistence across reloads
- **WHEN** the user switches themes and refreshes or revisits the page
- **THEN** the selected theme is restored immediately from `localStorage` before page rendering.

## ADDED Requirements

### Requirement: Unified Neumorphic Depth Tokens
The theme engine SHALL expose unified neumorphic elevation tokens that apply ambient drop shadows and specular top-rim highlights to cards across both Dark and Light themes.

#### Scenario: Neumorphic card rendering in Dark theme
- **WHEN** a card renders under `[data-theme="dark"]`
- **THEN** it displays a dual-layer box shadow combining ambient drop shadow (`0 6px 18px rgba(0, 0, 0, 0.65)`) with specular top-rim highlight (`inset 0 1px 0 rgba(255, 255, 255, 0.16)`) and translucent border creases.

#### Scenario: Neumorphic card rendering in Light theme
- **WHEN** a card renders under `[data-theme="light"]`
- **THEN** it displays a dual-layer box shadow combining ambient drop shadow (`0 4px 14px rgba(0, 0, 0, 0.08)`) with specular top-rim highlight (`inset 0 1px 0 rgba(255, 255, 255, 0.85)`).
