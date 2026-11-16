## ADDED Requirements

### Requirement: Dynamic WCH-Link Speed Tier Configuration
The system SHALL handle command `0x0C` by translating the speed tier byte into target clock frequency and reconfiguring the RVSWD PIO clock divider.

#### Scenario: Set RVSWD clock speed to 6 MHz
- **WHEN** host sends command `0x0C` with speed code 1
- **THEN** system configures RVSWD clock to 6000 kHz and returns status 0x01
