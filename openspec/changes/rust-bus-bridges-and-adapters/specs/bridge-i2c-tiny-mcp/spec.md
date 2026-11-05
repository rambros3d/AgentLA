## ADDED Requirements

### Requirement: I2C tiny-USB passthrough 
The firmware SHALL provide an emulation of Microchip MCP2221 USB-to-I2C interface with an endpoint-capable high-speed USB device, including reads/writes on pin owner claimed flexible pins.

#### Scenario: Set and read a register
- **WHEN** a host writes bytes via USB to I2C and then reads back
- **THEN** response is returned with bytes

### Requirement: Bridge pin ownership and safe claim
The firmware SHALL claim flexible pins through `pin_owner` and release them on disconnect or stop.

#### Scenario: Claim overlapping pin
- **WHEN** required flex pin is already claimed by another module
- **THEN** claim returns 409 and bus passthrough is not initialized
