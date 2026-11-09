## MODIFIED Requirements

### Requirement: DaisyUI and Responsive Layout Integration
The Servo Master application HTML template SHALL link `/daisyui.css` before `/style.css` and use native DaisyUI component classes arranged inside responsive grid containers styled according to the Industrial Design System (`data-style="industrial"`, `data-theme="nothing"` / `industrial-light`, NDot57 typography, dot-grid canvas, and zero emojis). The application SHALL feature a dedicated in-app sub-toolbar beneath `#top-bar` housing Gamepad status, Calibration Zero, and Emergency Disarm controls, and SHALL provide at least 72px bottom clearance padding to avoid occlusion by the host system dock when embedded.

#### Scenario: Rendering servo channels on desktop viewport
- **WHEN** the Servo Master application is displayed on a screen wider than 1024px
- **THEN** servo channel cards display side-by-side in responsive columns within a CSS grid and inherit active industrial system theme tokens.

#### Scenario: Embedded iframe sub-toolbar access
- **WHEN** the Servo Master application is embedded inside the AgentLA OS launcher iframe
- **THEN** Gamepad connection, Zero All, and Emergency Disarm buttons remain accessible in the sub-toolbar while `#top-bar` is suppressed.
