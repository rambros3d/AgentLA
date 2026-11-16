## MODIFIED Requirements

### Requirement: DAP_Transfer Command
The system SHALL perform SWD read/write operations via DAP_Transfer, supporting standard DP/AP transfers, posted read pipelines, and value matching (`MATCH_MASK` and `MATCH_VALUE`).

#### Scenario: Read DP register
- **WHEN** host sends DAP_Transfer to read DPIDR
- **THEN** system returns 32-bit IDCODE value

#### Scenario: Write AP register
- **WHEN** host sends DAP_Transfer to write AP register
- **THEN** system performs SWD write and returns ACK status

#### Scenario: Register value matching
- **WHEN** host sends DAP_Transfer with `MATCH_MASK` and `MATCH_VALUE` flags set
- **THEN** the system repeatedly polls the specified register until `(data & match_mask) == match_value` or the retry limit expires, consuming the 4-byte match payload without packet desynchronization
