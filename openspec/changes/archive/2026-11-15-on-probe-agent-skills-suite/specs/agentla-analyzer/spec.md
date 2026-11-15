## ADDED Requirements

### Requirement: Pin Arbiter Protocol Compliance
The `agentla-analyzer` skill SHALL require all protocol sniffing and logic analyzer operations to request and release pin reservations via `pins_claim` and `pins_release`.

#### Scenario: Enforce pin claim before sampling
- **WHEN** the agent initiates a logic capture or bus sniff on probe pins
- **THEN** it executes `pins_claim` specifying the pin mask and subsystem name, failing gracefully if the pins are busy.

### Requirement: Protocol Decoding and Bus Capture Playbooks
The `agentla-analyzer` skill SHALL guide the agent through configuring sample rates, triggers, and decoding protocols for I2C, SPI, UART, and CAN buses.

#### Scenario: Capture and decode I2C transaction
- **WHEN** an agent investigates an unresponsive I2C sensor
- **THEN** it sets sample rate, enables SDA/SCL pull-ups if needed, invokes `mcp_sniff_bus` with `protocol="i2c"`, and inspects ACK/NACK status and transaction bytes.

### Requirement: CAN Bus Termination and Packet Injection
The `agentla-analyzer` skill SHALL instruct the agent on controlling the 120Ω bus termination relay and injecting test frames safely.

#### Scenario: Automotive CAN bus verification
- **WHEN** analyzing a high-speed CAN network
- **THEN** the skill guides the agent to check bus termination state, engage the internal 120Ω relay if required, and verify bus traffic before injecting test frames.
