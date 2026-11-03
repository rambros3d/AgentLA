## MODIFIED Requirements

### Requirement: Linear Bento Grid Layout and Micro-Borders
The dashboard and workspace containers SHALL style instrument tiles, telemetry widgets, and inspector panels as flat Bento cards using DaisyUI `.card` surfaces with 1px borders (`border-base-300`), eliminating dual-source neumorphic drop shadows, inner inset shadows, and top specular rim highlights.

#### Scenario: Viewing dashboard tiles
- **WHEN** the dashboard is loaded
- **THEN** cards display with crisp 1px borders (`var(--color-base-300)`), flat resting surface backgrounds (`var(--color-base-200)`), no specular white rim highlights, and subtle ambient elevation only on hover

## REMOVED Requirements

### Requirement: Technical Millimeter Drafting Grid
**Reason**: Drafting grid linear gradients clash with high-contrast light mode palettes (`emerald`, `caramellatte`) and introduce visual noise contrary to pure DaisyUI design principles.
**Migration**: Body background renders as a clean, solid `var(--color-base-100)` surface without background grid lines or SVG noise overlays.
