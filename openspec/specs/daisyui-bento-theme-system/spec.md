# daisyui-bento-theme-system Specification

## Purpose
TBD - created by archiving change daisyui-bento-paper-themes. Update Purpose after archive.
## Requirements
### Requirement: DaisyUI Black and Emerald Semantic Color Tokens
The WebUI stylesheet SHALL implement DaisyUI v5 semantic tokens using native `oklch()` color values for both dark mode (`black` theme) and light mode (`emerald` theme).

#### Scenario: Dark theme active
- **WHEN** user selects dark mode (`[data-theme="dark"]`)
- **THEN** `--color-base-100` resolves to `oklch(0% 0 0)`, `--color-base-200` resolves to `oklch(19% 0 0)`, and text resolves to `oklch(87.609% 0 0)`

#### Scenario: Light theme active
- **WHEN** user selects light mode (`[data-theme="light"]`)
- **THEN** `--color-base-100` resolves to `oklch(100% 0 0)`, `--color-base-200` resolves to `oklch(93% 0 0)`, and text resolves to `oklch(35.519% 0.032 262.988)`

### Requirement: Linear Bento Grid Layout and Micro-Borders
The dashboard and workspace containers SHALL style instrument tiles, telemetry widgets, and inspector panels as Bento cards with 1px razor-sharp micro-borders, subtle top specular rim highlights, and unified corner radii.

#### Scenario: Viewing dashboard tiles
- **WHEN** the dashboard is loaded
- **THEN** cards display with crisp 1px borders, subtle inner inset shadows, and proportional bento sizing

### Requirement: Technical Millimeter Drafting Grid
The workspace canvas SHALL feature an ultra-subtle millimeter engineering drafting grid background.

#### Scenario: Inspecting the background canvas
- **WHEN** the workspace view is visible
- **THEN** the background renders a geometric grid pattern using low-opacity ink lines in both dark and light modes

