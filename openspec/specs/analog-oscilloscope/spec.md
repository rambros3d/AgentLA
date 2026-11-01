# analog-oscilloscope Specification

## Purpose
TBD - created by archiving change flexible-pin-instrument-suite. Update Purpose after archive.
## Requirements
### Requirement: 4-Channel Analog Oscilloscope Instrument
The system SHALL provide a dedicated Analog Scope instrument that samples the RP2350 SAR ADC on GPIO 26–29 (ADC 0–3) and renders real-time voltage curves with voltage divisions and trigger controls.

#### Scenario: Launching the analog oscilloscope
- **WHEN** user opens the Analog Scope instrument
- **THEN** system acquires an ADC claim on GP26–29, samples at up to 500 kS/s, and renders continuous analog waveforms from 0.0V to 3.3V

### Requirement: Dual-Personality Analog/Digital Lifecycle
The system SHALL return GPIO 26–29 to the flexible digital pool whenever the Analog Scope instrument is inactive.

#### Scenario: Closing the oscilloscope
- **WHEN** user stops the scope or selects a digital instrument
- **THEN** system releases the ADC claim handle, allowing GP26–29 to be claimed by digital adapters or logic analyzer banks

