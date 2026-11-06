# ws2812-timing-analyzer Specification

## ADDED Requirements

### Requirement: Addressable LED Pattern Generation
The system SHALL provide a PIO transmitter capable of generating standard 800 kHz WS2812B/SK6812 LED patterns on any flexible pin.

#### Scenario: Driving LED rainbow test pattern
- **WHEN** user selects GP15 and sends `POST /api/v1/led/pattern` with pattern `rainbow` and count `8`
- **THEN** system outputs valid 800 kHz GRB waveform data on GP15

### Requirement: LED Line Timing Margin Analysis
The system SHALL measure high and low pulse durations ($T_{0H}, T_{1H}, T_{0L}, T_{1L}$, and Reset) on incoming LED signals using high-resolution PIO capture and evaluate conformance to timing tolerances.

#### Scenario: Analyzing LED signal timing
- **WHEN** host sends `GET /api/v1/led/analyze?pin=15`
- **THEN** system samples incoming pulses, calculates measured $T_{0H}$ and $T_{1H}$ durations in nanoseconds, and returns PASS/FAIL status based on whether timings are within $\pm 150\,\text{ns}$ tolerance bands
