## MODIFIED Requirements

### Requirement: Interactive Probe Pins Bar
The Scope view SHALL render an interactive pins bar displaying all observation-capable hardware pins (`GP10` through `GP29`) across the top of the waveform window, with real-time visual status badges indicating their configured mode and trace color. Pin labeling and rendering SHALL sanitize text via an HTML escaping utility to prevent unhandled reference exceptions during page initialization.

#### Scenario: Pin status badge display
- **WHEN** pins are rendered in the top pins bar
- **THEN** unconfigured pins display a subtle muted outline
- **AND** active digital pins display a circular status badge filled with the channel's assigned trace color
- **AND** active analog pins (`GP26`–`GP29`) display a diamond badge (`◈`) with an `ADC` tag

#### Scenario: Opening pin configuration
- **WHEN** the user clicks any pin button in the top pins bar
- **THEN** the Interactive Pin Configuration Modal (`#modal-pin-config`) opens focused on that pin
