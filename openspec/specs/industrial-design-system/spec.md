# industrial-design-system Specification

## Purpose
Provides the Industrial design language and DaisyUI themes (`nothing` dark and `industrial-light` light) with OLED black / Technical Alabaster canvas, sharp 0.5rem geometry, adaptive 16px dot-grid matrix background, and Signal Red (`#D71921`) accents.

## Requirements

### Requirement: Default Industrial Theme and Persona
The system SHALL adopt the Industrial design language and the `nothing` DaisyUI theme as the sole, permanent presentation across all WebUI surfaces, eliminating runtime persona switching and retiring the Bento style.

#### Scenario: First boot default style
- **WHEN** a user loads the AgentLA WebUI
- **THEN** the root HTML tag has `data-style="industrial"` and `data-theme="nothing"`.

#### Scenario: Industrial color palette tokens
- **WHEN** the `nothing` theme is active
- **THEN** the base canvas is OLED black (`#000000`), card surfaces are `#0a0a0a`, structural borders are `#1a1a1a`, text is crisp off-white (`#f4f4f4`), and the primary accent is Signal Red (`#D71921`).

#### Scenario: Industrial Light theme tokens
- **WHEN** the `industrial-light` (or `[data-theme="light"]`) theme is active
- **THEN** the base canvas is pure white (`#ffffff`), card surfaces are `#f5f5f5`, structural borders are `#e5e5e5`, text is deep black (`#0f0f0f`), the background dot matrix renders with subtle dark dots (`rgba(0,0,0,0.12)`), and the primary accent remains Signal Red (`#D71921`).

### Requirement: Technical Dot-Grid Canvas Background
Under the industrial persona, the background canvas SHALL display an adaptive technical 16px dot-grid matrix rendered via pure CSS radial gradients without external image assets.

#### Scenario: Rendering industrial canvas
- **WHEN** `data-style="industrial"` is set on `<html>`
- **THEN** the viewport background displays a repeated 16px dot matrix with adaptive opacity matching base content luminance (`rgba(255, 255, 255, 0.12)` in dark mode, `rgba(0, 0, 0, 0.12)` in light mode).

### Requirement: Sharp Industrial Card and Widget Geometry
Under the industrial persona, cards, squircle tiles, and modal dialogs SHALL render with sharp 0.5rem corner radiuses and crisp 1px borders, eliminating heavy drop shadows.

#### Scenario: Card geometry in industrial mode
- **WHEN** elements with `.card`, `.stat`, or `.os-squircle-tile` render under `data-style="industrial"`
- **THEN** they display a 0.5rem corner radius, a 1px solid border, and zero box-shadow blur.

### Requirement: Permanent Retirement of Bento Persona
The WebUI SHALL NOT include alternative Bento styling rules, bubbled 1.25rem corner curves, or consumer glassmorphic drop shadows, and SHALL NOT display an Interface Persona switcher in System Settings.

#### Scenario: Absence of persona switcher
- **WHEN** user opens System Settings (`#modal-settings`)
- **THEN** no interface persona dropdown is displayed and the Industrial theme is enforced globally.
