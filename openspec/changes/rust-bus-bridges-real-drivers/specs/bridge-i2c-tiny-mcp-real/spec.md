## ADDED Requirements
### Requirement: I2C bridge REST claim
The firmware SHALL expose REST endpoints for I2C bridge on/off with proper JSON error bodies.
#### Scenario: claim conflict
- **WHEN** pin 16/17 are claimed by Servo
- **THEN** POST on returns 409, state remains idle
#### Scenario: bridge state query
- **WHEN** GET `/api/v1/bridge/i2c/on`
- **THEN** server responds status JSON
