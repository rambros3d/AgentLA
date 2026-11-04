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
The dashboard and workspace containers SHALL style instrument tiles, telemetry widgets, and inspector panels as flat Bento cards using DaisyUI `.card` surfaces with 1px borders (`border-base-300`), eliminating dual-source neumorphic drop shadows, inner inset shadows, and top specular rim highlights.

#### Scenario: Viewing dashboard tiles
- **WHEN** the dashboard is loaded
- **THEN** cards display with crisp 1px borders (`var(--color-base-300)`), flat resting surface backgrounds (`var(--color-base-200)`), no specular white rim highlights, and subtle ambient elevation only on hover

