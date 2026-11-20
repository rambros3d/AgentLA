## ADDED Requirements

### Requirement: Servo Master Standardized Horn Visualizer and Pure-Knob Dials
The Servo Master application SHALL integrate `agentla-servo-view.js` for parametric horn visualization and `pureknob.min.js` for precision channel angle dials.

#### Scenario: Dragging channel rotary dial
- **WHEN** user adjusts a servo channel angle dial
- **THEN** rotary dial updates the horn angle and dispatches the PWM pulse width to the hardware

### Requirement: Servo Master Multi-Column Widescreen Layout
The Servo Master interface SHALL display multi-channel servo strips in a responsive multi-column layout on wide screens.

#### Scenario: Multi-channel grid on wide screen
- **WHEN** user views Servo Master on a wide screen
- **THEN** servo channels and timeline sequencer arrange in a 12-column side-by-side grid rather than a single vertical list
