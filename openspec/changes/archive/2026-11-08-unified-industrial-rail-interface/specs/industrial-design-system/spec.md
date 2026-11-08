## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Permanent Retirement of Bento Persona
The WebUI SHALL NOT include alternative Bento styling rules, bubbled 1.25rem corner curves, or consumer glassmorphic drop shadows, and SHALL NOT display an Interface Persona switcher in System Settings.

#### Scenario: Absence of persona switcher
- **WHEN** user opens System Settings (`#modal-settings`)
- **THEN** no interface persona dropdown is displayed and the Industrial theme is enforced globally.
