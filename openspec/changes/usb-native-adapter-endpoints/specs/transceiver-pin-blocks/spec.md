## ADDED Requirements

### Requirement: Fixed Physical Transceiver Pin Assignments
The firmware capability model SHALL reserve dedicated physical pins for differential bus transceivers.

#### Scenario: Inspecting CAN pins
- **WHEN** user inspects the pin registry
- **THEN** GPIO 18 is designated as `CAN_TX`, GPIO 19 is designated as `CAN_RX`, and GPIO 22 controls $120\,\Omega$ termination

#### Scenario: Inspecting RS-485 pins
- **WHEN** user inspects the pin registry
- **THEN** GPIO 12 is designated as `UART0_TX`, GPIO 13 is designated as `UART0_RX`, and GPIO 14 controls driver enable (`DE_RE`)
