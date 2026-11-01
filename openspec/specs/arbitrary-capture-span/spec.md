# arbitrary-capture-span Specification

## Purpose
TBD - created by archiving change flexible-pin-instrument-suite. Update Purpose after archive.
## Requirements
### Requirement: Arbitrary Base Pin and Width Selection
The Logic Analyzer interface SHALL allow users to select any start pin from GPIO 10 to GPIO 22 and any channel width from 1 to 8 channels.

#### Scenario: Configuring a 4-channel span at GP14
- **WHEN** user sets base pin to GP14 and channel count to 4
- **THEN** system configures the capture bank to span GP14, GP15, GP16, and GP17 and requests `CAPTURECLAIM pins=14,15,16,17`

### Requirement: Digital Scope Measurement Computations
The Digital Scope mode SHALL calculate real-time timing statistics (frequency, period, pulse width high, pulse width low, and duty cycle) from captured edge transitions on any active digital channel.

#### Scenario: Displaying timing statistics
- **WHEN** a digital signal with alternating transitions is captured
- **THEN** the UI displays calculated frequency (in Hz/kHz/MHz) and duty cycle percentage on the channel readout

### Requirement: Capture Mode Configuration
The system SHALL support selecting between linear raw capture, Mode A (PIO RLE Streaming), Mode B (Dual-Core Turbo Compression), and Mode C (Circular Oscilloscope Pre/Post-Trigger) via the `CAPTUREMODE` command and WebUI control surface.

#### Scenario: Switching capture mode to Stream
- **WHEN** user executes `CAPTUREMODE STREAM`
- **THEN** the analyzer reconfigures PIO to load the `pio_rle_capture` program and sets the streaming endpoint to RLE packet processing

#### Scenario: Switching capture mode to Scope
- **WHEN** user executes `CAPTUREMODE SCOPE pre=64k post=384k`
- **THEN** the analyzer configures DMA Channel 0 for 64 KB circular ring buffering and DMA Channel 1 for 384 KB linear post-trigger capture

