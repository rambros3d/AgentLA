## MODIFIED Requirements

### Requirement: Responsive Active Bench Layout
The primary waveform observation view (renamed to "Scope") SHALL provide a unified Mixed-Signal Oscilloscope (MSO) window displaying digital logic channels in a stacked multi-lane layout in the top subwindow and analog channels superimposed on a shared 8 × 10 division graticule in the bottom subwindow, separated by a draggable horizontal splitter.

#### Scenario: Digital-only observation layout
- **WHEN** zero analog ADC pins are configured
- **THEN** the digital scope subwindow expands to fill 100% of the observation window height and the analog subwindow and splitter are hidden

#### Scenario: Mixed-signal observation with draggable splitter
- **WHEN** at least one analog ADC pin is configured alongside digital channels
- **THEN** the observation window renders both digital and analog subwindows separated by a draggable splitter bar (`.mso-window-splitter`)
- **AND** dragging the splitter smoothly resizes the relative heights of digital and analog subwindows

#### Scenario: Superimposed analog channels on shared graticule
- **WHEN** multiple analog ADC channels are active
- **THEN** all analog channels are drawn superimposed on the same 8 × 10 division graticule sharing calibrated voltage (`Volts/div`) and timebase (`Time/div`) axes

### Requirement: Docked Stimulus Deck
The Scope view SHALL provide a collapsible bottom Signal Generator drawer that minimizes to a compact status footer by default (`[ Signal Generator: GP12 @ 12.0 kHz ▴ ]`) and expands upward on click to expose frequency, duty cycle, and pin controls without obstructing waveform visibility.

#### Scenario: Minimized signal generator footer
- **WHEN** the Scope view loads
- **THEN** the Signal Generator renders minimized as a single-line footer bar, preserving over 90% of vertical height for the waveform canvases

#### Scenario: Expanding signal generator drawer
- **WHEN** the user clicks the minimized Signal Generator footer bar
- **THEN** the drawer smoothly expands upward displaying PWM frequency, duty cycle, and pin selectors
