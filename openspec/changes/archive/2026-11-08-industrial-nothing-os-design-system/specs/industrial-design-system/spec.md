## ADDED Requirements

### Requirement: Default Industrial Theme and Persona
The system SHALL adopt the Nothing OS-inspired Industrial persona (`data-style="industrial"`) and the `nothing` DaisyUI theme as the default presentation when no user preference is configured in `localStorage`.

#### Scenario: First boot default style
- **WHEN** a user loads the AgentLA WebUI with a clean cache
- **THEN** the root HTML tag has `data-style="industrial"` and `data-theme="nothing"`.

#### Scenario: Industrial color palette tokens
- **WHEN** the `nothing` theme is active
- **THEN** the base canvas is OLED black (`#000000`), card surfaces are `#0a0a0a`, structural borders are `#1a1a1a`, text is crisp off-white (`#f4f4f4`), and the primary accent is Nothing Signal Red (`#D71921`).

### Requirement: Technical Dot-Grid Canvas Background
Under the industrial persona, the background canvas SHALL display a technical 16px dot-grid matrix rendered via pure CSS radial gradients without external image assets.

#### Scenario: Rendering industrial canvas
- **WHEN** `data-style="industrial"` is set on `<html>`
- **THEN** the viewport background displays a repeated 16px dot matrix with subtle opacity (`rgba(255, 255, 255, 0.12)`).

### Requirement: Sharp Industrial Card and Widget Geometry
Under the industrial persona, cards, squircle tiles, and modal dialogs SHALL render with sharp 0.5rem corner radiuses and crisp 1px borders, eliminating heavy drop shadows.

#### Scenario: Card geometry in industrial mode
- **WHEN** elements with `.card`, `.stat`, or `.os-squircle-tile` render under `data-style="industrial"`
- **THEN** they display a 0.5rem corner radius, a 1px solid border (`rgba(255, 255, 255, 0.12)`), and zero box-shadow blur.
