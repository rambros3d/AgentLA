## ADDED Requirements

### Requirement: DaisyUI Scope Container and Telemetry Components
The Analog Oscilloscope interface SHALL wrap the waveform canvas inside a DaisyUI `.card .card-border .card-body` container and present signal measurements (Vpp, Vrms, Frequency, Duty) using DaisyUI `.stats .stat` blocks with `.stat-title`, `.stat-value`, and `.stat-desc`.

#### Scenario: Viewing oscilloscope measurements
- **WHEN** the user opens the Analog Scope panel
- **THEN** the scope canvas is framed within a DaisyUI `.card` and live voltage telemetry is formatted inside `.stats .stat` components without custom measurement wrapper divs
