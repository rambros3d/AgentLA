## ADDED Requirements

### Requirement: Universal target identification
The system SHALL identify a connected target's vendor, family, protection level, and factory geometry before any destructive operation.

#### Scenario: Identify STM32 target
- **WHEN** a target is connected via SWD and `TARGET IDENTIFY` is issued
- **THEN** the system reports vendor, family, part name, flash base/size, RAM base/size, and protection level

#### Scenario: Unknown target rejection
- **WHEN** identification fails to match a known vendor/family
- **THEN** the system returns `ERR TARGET UNKNOWN` and forbids erase/program operations

### Requirement: Protection level detection
The system SHALL report the target's protection level and whether mass-erase is permitted.

#### Scenario: RDP level 1 on STM32
- **WHEN** an STM32 target with RDP level 1 is identified
- **THEN** the system reports `protection=rdp1` and `mass_erase_allowed=true`

#### Scenario: RDP level 2 on STM32
- **WHEN** an STM32 target with RDP level 2 is identified
- **THEN** the system reports `protection=rdp2` and `recoverable=false`

### Requirement: Factory geometry readback
The system SHALL prefer factory-reported flash/RAM sizes over hardcoded defaults.

#### Scenario: Read STM32 flash size from factory row
- **WHEN** target flash size is queried
- **THEN** the system reads factory info (STM32 F1 option bytes / F4+ factory row) before falling back to compiled defaults