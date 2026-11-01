# internal-bus-sniffer Specification

## Purpose
TBD - created by archiving change flexible-pin-instrument-suite. Update Purpose after archive.
## Requirements
### Requirement: 1-Tap Internal Bus Sniffer Preset
The system SHALL provide 1-tap presets to automatically configure an adapter driver and logic analyzer observer on identical physical pins in split-view mode.

#### Scenario: Activating I2C bus sniffer
- **WHEN** user clicks the "Sniff I2C" split preset
- **THEN** system claims GP10/11 for I2C Master, claims GP10/11 for Logic Analyzer observer, mounts I2C in Slot 1 and Logic Analyzer in Slot 2, and decodes transmitted packets in real-time without external loopback jumpers

