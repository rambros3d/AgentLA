## ADDED Requirements

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
