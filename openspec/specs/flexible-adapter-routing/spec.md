# flexible-adapter-routing Specification

## Purpose
TBD - created by archiving change flexible-pin-instrument-suite. Update Purpose after archive.
## Requirements
### Requirement: Interactive Pin Selection for Bus Adapters
The WebUI SHALL allow users to select and reassign valid pin pairs within GPIO 10–29 for I2C, SPI, and UART adapters.

#### Scenario: Selecting alternative I2C pins
- **WHEN** user chooses pins GP14 and GP15 for the I2C adapter
- **THEN** system releases any active I2C claim handle, requests `PINCLAIM pins=14,15 func=3 observer=0`, and reinitializes the hardware adapter on the new pins

#### Scenario: Attempting claim on fixed programmer pins
- **WHEN** user attempts to route an adapter to GPIO 0–9
- **THEN** system rejects the selection with a validation error indicating pins 0–9 are reserved for the programmer

