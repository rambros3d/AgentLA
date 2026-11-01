## MODIFIED Requirements

### Requirement: Field-derived command invocation
The WebUI SHALL collect user-supplied values and convert them to canonical command names and argument strings before dispatch, parsing JSON-RPC results to validate operation success before progressing through multi-step workflows.

#### Scenario: Flash inputs and chunk verification
- **WHEN** a user sets target address and selects a firmware file
- **THEN** the Flasher SHALL construct canonical `FLASH ERASE`, `FLASH WRITE`, and `FLASH VERIFY` requests and evaluate `result.stdout` to drive progress

#### Scenario: Sequential I2C write and read
- **WHEN** a user requests a combined I2C write and read operation
- **THEN** the interface SHALL dispatch sequential `I2C_WRITE` and `I2C_READ` commands referencing the active adapter claim handle
