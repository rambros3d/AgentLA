## ADDED Requirements

### Requirement: Multi-Channel ADC Scope Live Verification
The system SHALL support automated validation of ADC channels (ADC0 on GPIO 26, ADC1 on GPIO 27, ADC2 on GPIO 28) with `ADCSCOPE pin=<pin> samples=<count>` command and verify that voltage metrics fall within calibrated 0.0V–3.3V bounds.

#### Scenario: Live ADC Scope acquisition check
- **WHEN** the host issues `ADCSCOPE pin=26 samples=100` while connected to the target reference signal
- **THEN** the system returns valid voltage readings (CH0, CH1, CH2, CH3) with RAW sample conversions and non-zero peak-to-peak amplitude
