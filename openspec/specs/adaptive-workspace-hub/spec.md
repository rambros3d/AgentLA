# adaptive-workspace-hub Specification

## Purpose
TBD - created by archiving change adaptive-touch-desktop-webui. Update Purpose after archive.
## Requirements
### Requirement: Touch and Desktop Adaptive Navigation Shell
The WebUI SHALL provide an adaptive viewport shell using dynamic viewport units (`100dvh`, `100dvw`) that automatically presents a thumb-friendly Bottom Navigation Bar with $\ge 48\text{px} \times 48\text{px}$ touch targets on viewports below 900px, and transitions to a compact Left Navigation Rail on viewports 900px and wider.

#### Scenario: Mobile and touch portrait layout
- **WHEN** the WebUI is accessed on a viewport narrower than 900px or in touch portrait mode
- **THEN** the navigation bar renders anchored at the bottom with touch-sized icon-and-label tabs, while the active instrument fills the remaining viewport height without window chrome.

#### Scenario: Desktop and landscape wide layout
- **WHEN** the WebUI is accessed on a display 900px or wider
- **THEN** the navigation renders as an icon rail on the left margin, maximizing horizontal and vertical canvas width for instrument displays.

### Requirement: Unified Dashboard Overview
The WebUI SHALL display a Unified Dashboard as the default landing view upon loading, displaying probe health telemetry, an interactive 30-pin domain allocation map (`GPIO 0–29`), and 1-tap launcher tiles.

#### Scenario: Landing view display
- **WHEN** a user navigates to `http://agentla.lab`
- **THEN** the Dashboard workspace opens, showing probe CPU frequency, voltage status, USB mode, active target detection, and instrument launch tiles.

#### Scenario: Real-time pin domain allocation visualization
- **WHEN** any subsystem or adapter claims pins
- **THEN** the 30-pin allocation widget visually updates the pin state, function role (`SWD`, `I2C`, `I3C`, `UART`, `SPI`, `CAPTURE`), and owner handle.

### Requirement: Slide-Up Bottom Sheet and Drawer Inspection
The WebUI SHALL display tool configuration, trigger setup, and pin selection within a slide-up Bottom Sheet on mobile viewports and a slide-out Right Drawer on desktop viewports, replacing floating dialog windows.

#### Scenario: Mobile tool configuration
- **WHEN** a user taps "Configure" on a mobile viewport
- **THEN** a bottom sheet slides up from the viewport base with large form controls and closes upon downward swipe or outside tap.

#### Scenario: Desktop configuration drawer
- **WHEN** a user opens configuration on a desktop viewport
- **THEN** an inspector drawer slides out from the right margin without obscuring the primary waveform or data log.

### Requirement: Waveform Multi-Touch Navigation
The Logic Analyzer canvas SHALL support native touch gestures using `touch-action: none` for horizontal pinch-to-zoom, one/two-finger panning, and double-tap zoom reset.

#### Scenario: Touch pinch-to-zoom and pan
- **WHEN** a user pinches or drags across the waveform canvas on a touch device
- **THEN** the sample buffer zooms in/out horizontally and pans through time without triggering browser page-level zooming or scrolling.

#### Scenario: Mouse wheel zoom and drag
- **WHEN** a user uses mouse wheel or drag on desktop
- **THEN** the waveform zooms centered at the cursor and pans horizontally.

