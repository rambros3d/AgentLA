## ADDED Requirements

### Requirement: Personality Mode Selection
The system SHALL support selecting between CMSIS-DAP, ST-Link, and WCH-Link programmer personalities at runtime with persistence across reboots.

#### Scenario: Select personality via control command
- **WHEN** user or host sends a personality switch command
- **THEN** system saves the new personality in non-volatile storage and activates the corresponding USB debug endpoints

#### Scenario: Query active personality
- **WHEN** user queries current programmer personality state
- **THEN** system returns the active personality identifier and target protocol status
