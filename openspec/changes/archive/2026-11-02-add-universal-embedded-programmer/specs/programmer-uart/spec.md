## ADDED Requirements

### Requirement: Bootloader recovery orchestration
The system SHALL define bootloader programming as an orchestrated sequence using BOOT0/nRST control and UART communication, rather than raw protocol commands only.

#### Scenario: Orchestration sequence
- **WHEN** bootloader programming is requested
- **THEN** the system enters bootloader mode, performs sync/erase/program/verify, and returns to application mode

#### Scenario: Orchestration failure cleanup
- **WHEN** any step in the orchestrated sequence fails
- **THEN** the system releases BOOT0, resets the target, and returns a descriptive error