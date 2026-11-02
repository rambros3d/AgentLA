# webui-protocol-overlays Specification

## Purpose
Visual annotation bubbles and protocol decode controls on the logic analyzer and scope canvas.
## Requirements
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

### Requirement: Integrated Protocol Decoder Controls
The WebUI SHALL display protocol decoder configuration and state within an integrated decoder bar positioned directly above the digital scope lanes (`[ ☍ Decoder: I2C (SDA: D0, SCL: D1) ⚙ ▾ ] [ 📋 Packet Table ]`), providing immediate access to protocol selection, channel mapping, baud/clock settings, and a slide-out packet list drawer.

#### Scenario: Decoder controls placement
- **WHEN** the Scope view is active and digital channels are displayed
- **THEN** the protocol decoder bar renders directly above the digital waveform lanes without obstructing the top toolbar or the waveform canvas

#### Scenario: Packet table drawer toggle
- **WHEN** the user clicks `[ 📋 Packet Table ]` in the decoder bar
- **THEN** an interactive drawer or panel displays a chronologically indexed list of decoded packets with timestamp, direction, address, data payload, and parity/CRC status

#### Scenario: In-canvas bubble hover tooltip
- **WHEN** the user hovers over a decoded packet bubble on the digital waveform canvas
- **THEN** a detailed tooltip displays the decoded field semantics, raw binary/hex values, timing duration, and sample range

