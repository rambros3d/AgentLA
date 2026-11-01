## ADDED Requirements

### Requirement: Capture Mode Configuration
The system SHALL support selecting between linear raw capture, Mode A (PIO RLE Streaming), Mode B (Dual-Core Turbo Compression), and Mode C (Circular Oscilloscope Pre/Post-Trigger) via the `CAPTUREMODE` command and WebUI control surface.

#### Scenario: Switching capture mode to Stream
- **WHEN** user executes `CAPTUREMODE STREAM`
- **THEN** the analyzer reconfigures PIO to load the `pio_rle_capture` program and sets the streaming endpoint to RLE packet processing

#### Scenario: Switching capture mode to Scope
- **WHEN** user executes `CAPTUREMODE SCOPE pre=64k post=384k`
- **THEN** the analyzer configures DMA Channel 0 for 64 KB circular ring buffering and DMA Channel 1 for 384 KB linear post-trigger capture
