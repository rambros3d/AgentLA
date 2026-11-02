# interactive-pinout-svg Specification

## Purpose
TBD - created by archiving change dynamic-firmware-webui. Update Purpose after archive.
## Requirements
### Requirement: Dynamic Pinout SVG Inlining
The WebUI SHALL asynchronously fetch the board-specific `pinout.svg` and inline the vector markup directly into the DOM instead of rendering it as an isolated raster image or external `<img>` tag.

#### Scenario: Inlining vector pinout on initialization
- **WHEN** the board manifest is loaded and contains a valid `assets.pinout` path
- **THEN** the WebUI SHALL fetch `/pinout.svg` and insert its `<svg>` root element into `#hardware-pinout-canvas` inside the full-screen Pin Matrix view (`#view-pin-matrix`), arranged side-by-side with the routing table

### Requirement: Interactive Hover Synchronization
The WebUI SHALL bind pointer events to SVG elements with matching pin IDs (`pin-gp<N>`) to provide bidirectional hover highlighting with the pin matrix table.

#### Scenario: Hovering SVG pin pad
- **WHEN** the user moves the pointer over an SVG pin pad element with id `pin-gp4`
- **THEN** the SVG pin element SHALL receive the active highlight styling AND the corresponding row in the Pin Matrix table SHALL be highlighted and scrolled into view

#### Scenario: Hovering Pin Matrix table row
- **WHEN** the user moves the pointer over row `GP4` in the Pin Matrix table
- **THEN** the SVG element with id `pin-gp4` SHALL receive the matching highlight glow effect

### Requirement: Pin Click Configuration Modal
Clicking on an SVG pin pad or Pin Matrix row SHALL trigger an inspection modal detailing the pin's current hardware assignment, alternate muxing options, voltage domain, and electrical constraints.

#### Scenario: Clicking an interactive pin
- **WHEN** the user clicks an active pin pad in the SVG diagram or Pin Matrix
- **THEN** the WebUI SHALL open a pin details popover displaying the pin's zone, physical connector pin, default role, and voltage rail limits

