## ADDED Requirements

### Requirement: Target Voltage Acquisition and Noise Clamping
The system SHALL sample the target supply rail via ADC channel 2, convert the raw ADC value to millivolts, and clamp readings below 800 mV to 0 mV to denote an unpowered or disconnected target.

#### Scenario: Active target powered at 3.3V
- **WHEN** target rail is supplied with 3300 mV
- **THEN** system measures and returns a voltage between 3200 mV and 3400 mV

#### Scenario: Unpowered or disconnected target
- **WHEN** target rail is unpowered or floating with less than 800 mV
- **THEN** system clamps measured voltage to 0 mV
