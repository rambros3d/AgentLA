# analog-oscilloscope Specification

## MODIFIED Requirements

### Requirement: 4-Channel Analog Oscilloscope Instrument
The system SHALL provide a dedicated Analog Scope instrument that samples the RP2350 SAR ADC on GPIO 26–29 (ADC 0–3) using DMA and renders real-time voltage curves with voltage divisions, trigger controls, and live SSE streaming.

#### Scenario: Launching the analog oscilloscope
- **WHEN** user opens the Analog Scope instrument in the Scope Tab
- **THEN** system acquires an ADC claim on GP26–29, samples at up to 500 kS/s single-channel or 125 kS/s 4-channel round-robin, and streams continuous analog waveforms from 0.0V to 3.3V over SSE at `/api/v1/oscilloscope/stream`

## ADDED Requirements

### Requirement: Real-Time Signal Metrics Calculation
The firmware SHALL compute $V_{pp}, V_{rms}$, frequency, and duty cycle from the active ADC capture buffer.

#### Scenario: Displaying signal metrics
- **WHEN** a waveform capture trigger completes
- **THEN** the system calculates $V_{pp}, V_{rms}$, estimated frequency, and duty cycle and includes them in the SSE frame and REST snapshot `/api/v1/oscilloscope/data`
