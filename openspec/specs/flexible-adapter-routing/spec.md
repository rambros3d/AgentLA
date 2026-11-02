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

### Requirement: Master-Detail USB Protocol Adapters Layout
The WebUI USB Protocol Adapters view (`#view-adapters`) SHALL present hardware protocol adapters using a Master-Detail layout featuring a left master list of all 6 bridges (UART, I2C, I3C, SPI, CAN, Industrial) and a right detail workspace displaying full transaction controls and persistent logs for the selected protocol.

#### Scenario: Navigating between protocol adapters
- **WHEN** user clicks on a protocol in the left master list
- **THEN** the right detail workspace displays the configuration and transaction controls for that protocol while preserving the previous protocol's terminal log in the DOM

#### Scenario: Intelligent default adapter selection
- **WHEN** user navigates to the Adapters view
- **THEN** the active hardware profile's claimed adapter is selected by default, falling back to UART Bridge (CDC1) if no profile claims an adapter

#### Scenario: Executing quick preset transactions
- **WHEN** user clicks a quick preset chip (e.g., "Scan Bus" or "Read 24C02 EEPROM")
- **THEN** the corresponding transaction parameters populate and dispatch to the hardware bridge immediately

