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

### Requirement: Multi-Channel ADC Scope Live Verification
The system SHALL support automated validation of ADC channels (ADC0 on GPIO 26, ADC1 on GPIO 27, ADC2 on GPIO 28) with `ADCSCOPE pin=<pin> samples=<count>` command and verify that voltage metrics fall within calibrated 0.0V–3.3V bounds.

#### Scenario: Live ADC Scope acquisition check
- **WHEN** the host issues `ADCSCOPE pin=26 samples=100` while connected to the target reference signal
- **THEN** the system returns valid voltage readings (CH0, CH1, CH2, CH3) with RAW sample conversions and non-zero peak-to-peak amplitude

### Requirement: DaisyUI Scope Container and Telemetry Components
The Analog Oscilloscope interface SHALL wrap the waveform canvas inside a DaisyUI `.card .card-border .card-body` container and present signal measurements (Vpp, Vrms, Frequency, Duty) using DaisyUI `.stats .stat` blocks with `.stat-title`, `.stat-value`, and `.stat-desc`.

#### Scenario: Viewing oscilloscope measurements
- **WHEN** the user opens the Analog Scope panel
- **THEN** the scope canvas is framed within a DaisyUI `.card` and live voltage telemetry is formatted inside `.stats .stat` components without custom measurement wrapper divs

