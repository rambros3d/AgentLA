# unified-active-bench Specification

## Purpose
Specifies the combined observation and stimulus workbench layout, responsive breakpoints, docked stimulus deck, and synchronized "Send & Sniff" transaction capture.
## Requirements
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

### Requirement: Synchronized Send and Sniff
The system SHALL support an integrated "Send & Sniff" workflow where transmitting bus master commands automatically triggers signal observation.

#### Scenario: Auto-sniff armed on bus transmission
- **WHEN** the user transmits an I2C, SPI, UART, CAN, Modbus, 1-Wire, or LIN transaction and `Auto-Sniff` is active
- **THEN** the system SHALL immediately arm logic analyzer capture if idle, receive the transaction waveform, and center the timeline on the transmitted packet

#### Scenario: Auto-sniff toggling
- **WHEN** the user toggles the `Auto-Sniff on Transmit` toolbar button
- **THEN** the system SHALL update the auto-sniff preference and persist it in local storage

