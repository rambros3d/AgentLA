## ADDED Requirements

### Requirement: Interactive Protocol Canvas Annotations
The WebUI logic analyzer SHALL render interactive visual annotation bubbles aligned with sample timestamps for decoded protocol events (CAN, Modbus, DMX, Addressable LEDs, 1-Wire, I3C, and SWD).

#### Scenario: Visualizing decoded CAN frame
- **WHEN** CAN decode is active on a logic trace
- **THEN** the canvas renders a green annotation bubble displaying the Arbitration ID above the SOF timestamp, payload hex bytes along data bits, and CRC status at the CRC delimiter

### Requirement: CSS Color Swatches for Addressable LEDs
The WebUI SHALL display visual color swatches rendering the exact decoded RGB color for each addressable LED pixel.

#### Scenario: Displaying LED color swatch
- **WHEN** an addressable LED pixel is decoded with value `#FF5500`
- **THEN** the UI displays an inline visual color swatch filled with `#FF5500` alongside the hex value
