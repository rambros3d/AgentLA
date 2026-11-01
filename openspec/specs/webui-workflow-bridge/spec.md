# webui-workflow-bridge Specification

## Purpose
TBD
## Requirements
### Requirement: Field-derived command invocation
The WebUI SHALL collect user-supplied values and convert them to canonical command names and argument strings before dispatch, parsing JSON-RPC results to validate operation success before progressing through multi-step workflows.

#### Scenario: Flash inputs and chunk verification
- **WHEN** a user sets target address and selects a firmware file
- **THEN** the Flasher SHALL construct canonical `FLASH ERASE`, `FLASH WRITE`, and `FLASH VERIFY` requests and evaluate `result.stdout` to drive progress

#### Scenario: Sequential I2C write and read
- **WHEN** a user requests a combined I2C write and read operation
- **THEN** the interface SHALL dispatch sequential `I2C_WRITE` and `I2C_READ` commands referencing the active adapter claim handle

### Requirement: Progress panels
The WebUI SHALL update long-running operation controls only after receiving operation events.

#### Scenario: Progress state
- **WHEN** an operation updates
- **THEN** the browser SHALL reflect the latest handle, status, and completion percentage

### Requirement: Safety Confirmation
The WebUI SHALL require confirmation before exposing destructive firmware operations.

#### Scenario: Mass erase
- **WHEN** a user initiates mass erase
- **THEN** the UI SHALL show the exact operation and arguments before submitting it

