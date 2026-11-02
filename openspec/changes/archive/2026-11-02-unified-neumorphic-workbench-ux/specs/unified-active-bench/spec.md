## MODIFIED Requirements

### Requirement: Responsive Active Bench Layout
The WebUI SHALL display a unified active bench maximizing waveform canvas vertical height, featuring clustered observation controls below the canvas, floating canvas zoom controls, and a collapsible bottom Signal Generator drawer.

#### Scenario: Clustered observation toolbar rendering
- **WHEN** user views the Active Bench
- **THEN** observation controls render grouped into Capture (Run, Rate, Trigger), Analysis (Decode, Export, Auto-Sniff), and Hardware Bank claim clusters without horizontal overcrowding

#### Scenario: Toggling Signal Generator drawer
- **WHEN** user clicks the minimized Signal Generator footer bar
- **THEN** the drawer smoothly expands upward allowing PWM frequency and duty cycle adjustment while maintaining live waveform visibility
