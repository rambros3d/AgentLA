# daisyui-bento-theme-system Specification

## Purpose
DaisyUI semantic design tokens, bento card surfaces, and atomic layout utilities providing responsive, theme-consistent layouts across desktop and mobile workbenches.

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
The dashboard, workspace containers, and user-installed apps SHALL style instrument tiles, telemetry widgets, and inspector panels as flat Bento cards using DaisyUI `.card` surfaces with 1px borders (`border-base-300`), eliminating dual-source neumorphic drop shadows, inner inset shadows, and top specular rim highlights, and relying on the shared layout utility layer for responsive grid positioning.

#### Scenario: Viewing dashboard tiles
- **WHEN** the dashboard is loaded
- **THEN** cards display with crisp 1px borders (`var(--color-base-300)`), flat resting surface backgrounds (`var(--color-base-200)`), no specular white rim highlights, and subtle ambient elevation only on hover

### Requirement: Comprehensive Atomic Layout Utility Layer
The primary stylesheet `webui/style.css` SHALL supply a complete suite of Tailwind CSS-compatible layout utility classes covering Flexbox, Grid, Gaps, Spacing, Sizing, and Theme Opacity Mixes so user apps and workbench modules can construct responsive multi-column layouts without layout collapse.

#### Scenario: Rendering multi-column responsive grids
- **WHEN** an HTML view specifies classes `grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4`
- **THEN** elements arrange into 1, 2, 3, or 4 equal columns based on viewport breakpoint with 1rem (16px) gaps between tiles

#### Scenario: Flex alignment and spacing
- **WHEN** an element specifies `flex items-center justify-between gap-2 p-3`
- **THEN** it renders with CSS flex display, centered cross-axis alignment, space-between main-axis distribution, 8px element gap, and 12px padding
