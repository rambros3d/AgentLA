## ADDED Requirements

### Requirement: Raw I2C Stream Transfer Endpoint
The system SHALL provide an atomic HTTP REST endpoint `POST /api/v1/bus/i2c/transfer` to execute raw write and read transactions without register offset semantics.

#### Scenario: Raw write and read over I2C bus
- **WHEN** client issues `POST /api/v1/bus/i2c/transfer` with target 7-bit address, raw payload bytes, and requested read length
- **THEN** the system claims I2C bus pins from Pin Arbiter, transmits the raw payload bytes, reads the requested number of bytes, and releases the bus
- **AND** returns a JSON response containing `status`, bytes written, and hexadecimal received bytes

### Requirement: Raw I3C SDR Transfer Endpoint
The system SHALL provide an atomic HTTP REST endpoint `POST /api/v1/bus/i3c/transfer` to execute raw write and read transactions on the I3C bus.

#### Scenario: Raw write and read over I3C bus
- **WHEN** client issues `POST /api/v1/bus/i3c/transfer` with target address, dynamic address assignment request, and transfer payload
- **THEN** the system executes the I3C SDR transfer via `I3cBridge` and returns the received bytes in hexadecimal format
