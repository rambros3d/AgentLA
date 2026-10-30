# webui-workflow-bridge Specification

## Purpose
TBD

## Requirements
### Requirement: Field-derived command invocation
The WebUI SHALL collect user-supplied values and convert them to canonical command names and argument strings before dispatch.

#### Scenario: Flash inputs
- **WHEN** a user sets target address and selects a firmware file
- **THEN** the Flasher SHALL construct commands from those values instead of ignoring them

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
