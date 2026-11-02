## ADDED Requirements

### Requirement: BOOT0 pin control
The system SHALL control BOOT0 via a dedicated GPIO (default GPIO2, push-pull, board-configurable), alongside nRST (GPIO6), to support bootloader recovery.

#### Scenario: Assert BOOT0 high
- **WHEN** BOOT0 assertion is requested
- **THEN** the system drives the configured BOOT0 GPIO as required by the board option and returns `OK BOOT0 HIGH`

#### Scenario: Release BOOT0
- **WHEN** BOOT0 release is requested
- **THEN** the system releases the configured BOOT0 GPIO and returns `OK BOOT0 LOW`

### Requirement: Board-configurable BOOT0 pin
The system SHALL allow `board_settings.h` to select whether GPIO7 is BOOT0, power enable, or unused, to avoid conflicting with boards requiring GPIO7 for power.

#### Scenario: Power-enable configuration preserved
- **WHEN** a board configuration uses GPIO7 for target power enable
- **THEN** bootloader recovery is disabled for that build and documented accordingly