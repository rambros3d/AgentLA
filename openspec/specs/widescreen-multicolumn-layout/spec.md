# widescreen-multicolumn-layout Specification

## Purpose
TBD - created by archiving change webapp-widgets-and-widescreen-layout. Update Purpose after archive.
## Requirements
### Requirement: Responsive Multi-Column Wide-Screen Layout
The system web applications SHALL adopt a responsive multi-column layout on wide-screen displays (`lg:` breakpoint and above) using a 12-column grid (`grid grid-cols-1 lg:grid-cols-12 gap-4 items-stretch`) to utilize horizontal screen real estate without requiring excessive vertical scrolling.

#### Scenario: Viewing web application on wide screen
- **WHEN** user views `barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`, or `files.html` on a display width $\ge 1024\text{ px}$
- **THEN** controls, dials, and configuration panels occupy 4 to 5 columns on one side, and visualizers, canvases, scopes, or waveforms occupy 7 to 8 columns on the other side

#### Scenario: Viewing web application on narrow screen
- **WHEN** user views the application on a mobile or narrow display ($< 1024\text{ px}$)
- **THEN** columns stack vertically into a single-column scrollable stream

### Requirement: Full-Bleed Viewport Geometry
The application container and viewports SHALL remain full-bleed (`height: calc(100vh - 48px)` or `height: 100vh` when embedded) without dead margin shelves behind the dock.

#### Scenario: Full-bleed viewport check
- **WHEN** application is rendered standalone or inside an iframe
- **THEN** main content fills the entire vertical viewport down to the bottom dock

