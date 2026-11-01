# bench-theme-engine Specification

## Purpose
TBD - created by archiving change adaptive-touch-desktop-webui. Update Purpose after archive.
## Requirements
### Requirement: Deep Dark and High-Contrast Light Theme Switching
The WebUI SHALL provide a theme engine supporting both AgentLA's default Deep Dark theme (`#08111d` slate with cyan/blue accents) and a high-contrast Light Theme for bright laboratory environments.

#### Scenario: Default deep dark theme application
- **WHEN** the WebUI loads without existing user preference
- **THEN** the system applies the deep dark palette with dark background, high-contrast trace colors, and cyan accents.

#### Scenario: Toggle to light theme
- **WHEN** the user taps the theme toggle button in the top navigation bar
- **THEN** the root CSS variables instantly switch to clean high-contrast light colors with white/light gray surfaces and dark text without requiring page reload.

#### Scenario: Theme persistence across reloads
- **WHEN** the user switches themes and refreshes or revisits the page
- **THEN** the selected theme is restored immediately from `localStorage` before page rendering.

