## ADDED Requirements

### Requirement: JSON Factory Recipe Specification
The test system SHALL support declarative JSON test recipes defining the target microcontroller, firmware binary files, flash address offsets, electrical thresholds, serialization rules, and ordered functional circuit test (FCT) steps.

#### Scenario: Recipe validation on load
- **WHEN** a test recipe is uploaded to the WebUI or loaded by the CLI runner
- **THEN** the system SHALL validate the schema for `target`, `pre_flight_safety`, `serialization`, and `fct_steps` before enabling the automated run mode

### Requirement: Unique Board Serialization Rules
The test system SHALL support rule-based serialization to generate unique serial numbers and MAC addresses and patch them into the firmware image at a specified Flash/OTP offset.

#### Scenario: Injecting sequential serial number
- **WHEN** a unit reaches the programming step
- **THEN** the system SHALL generate the next serial number using the configured format (e.g. `MC26-{BATCH}-{SEQ:05d}`), write it to the designated flash offset, and increment the sequence counter
