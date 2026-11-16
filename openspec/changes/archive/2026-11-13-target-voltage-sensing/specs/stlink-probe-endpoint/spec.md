## ADDED Requirements

### Requirement: ST-Link Target Voltage Reporting (Command 0xF7)
The system SHALL respond to command `0xF7` (`STLINK_GET_TARGET_VOLTAGE`) with an 8-byte response containing reference factor 2400 (bytes 0..3) and the live target voltage in millivolts (bytes 4..7).

#### Scenario: Query target voltage when target is powered
- **WHEN** host issues command byte `0xF7` and target rail is at 3300 mV
- **THEN** system responds with 8 bytes: `put32(resp[0..4], 2400)` and `put32(resp[4..8], 3300)`

#### Scenario: Query target voltage when target is unpowered
- **WHEN** host issues command byte `0xF7` and target rail is at 0 mV
- **THEN** system responds with 8 bytes: `put32(resp[0..4], 2400)` and `put32(resp[4..8], 0)`
