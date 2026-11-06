## ADDED Requirements

### Requirement: Autonomous bus transaction sniffing
The system SHALL provide an autonomous MCP tool `mcp_sniff_bus` to sample target I2C or SPI lines for a specified duration and return a structured transaction inventory.

#### Scenario: I2C bus transaction sniff
- **WHEN** `mcp_sniff_bus` is invoked with protocol `i2c` and duration 200ms
- **THEN** the system samples the SDA/SCL lines, decodes transactions using the in-firmware decoder, and returns a JSON list of active slave addresses, read/write payloads, and ACK/NACK status

#### Scenario: Pin conflict rejection
- **WHEN** the bus pins are already claimed by an active driver or programmer session
- **THEN** the system returns `isError: true` with a description of the conflicting pin ownership
