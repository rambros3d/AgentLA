# digital-signal-generator Specification

## MODIFIED Requirements

### Requirement: Signal Generator Bench Instrument
The system SHALL provide a dedicated Signal Generator instrument allowing users to output PWM signals, clock references, and pulse bursts on any pin in GPIO 10–29 with REST control endpoints.

#### Scenario: Outputting a 100 kHz 50% duty clock
- **WHEN** user selects GP12, sets frequency to 100000 Hz, duty cycle to 50%, and sends `POST /api/v1/generator/start`
- **THEN** system claims GP12 for PWM output and configures the hardware PWM slice to drive the selected signal continuously

#### Scenario: Stopping signal output
- **WHEN** user sends `POST /api/v1/generator/stop` or navigates away
- **THEN** system stops the PWM slice and releases the pin back to the available flexible pool

## ADDED Requirements

### Requirement: Burst Pulse Generation
The system SHALL support generating a finite burst count of $N$ clock pulses on any flexible pin.

#### Scenario: Triggering an MCU recovery burst
- **WHEN** user requests a burst of 64 pulses at 8 MHz on GP14
- **THEN** system outputs exactly 64 pulses and automatically stops the slice output
