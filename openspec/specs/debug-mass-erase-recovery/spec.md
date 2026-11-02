# debug-mass-erase-recovery

## Purpose
Provides ADIv5 AP register write and hardware-assisted debug mass-erase recovery for protected ARM microcontrollers (such as STM32, Nordic, and WCH parts), clearing protection to Level 0 and verifying post-erase identity.

## Requirements

### Requirement: Debug mass-erase recovery for ARM targets
The system SHALL recover an RDP Level 1 ARM target by issuing a mass-erase through ADIv5 AP register writes without requiring a host tool.

#### Scenario: Successful mass erase
- **WHEN** the target is identified as RDP Level 1 and debug mass-erase is requested
- **THEN** the system performs the family-specific mass-erase sequence and reports `OK MASS ERASE DONE`

#### Scenario: Mass erase not permitted
- **WHEN** the target is RDP Level 2 or debug access is disabled
- **THEN** the system returns `ERR MASS ERASE NOT PERMITTED` and directs the user to bootloader recovery

### Requirement: Post-erase re-identification
The system SHALL re-identify the target after mass erase to confirm recovery.

#### Scenario: Erase succeeded
- **WHEN** mass erase completes
- **THEN** the system re-runs target identification and reports `protection=none` before any programming

### Requirement: Family-specific mass-erase procedures
The system SHALL implement mass-erase for STM32, Nordic, and WCH ARM-class targets without changing prior successful debug paths.

#### Scenario: STM32 mass erase
- **WHEN** the target family is STM32
- **THEN** the system uses the ST-defined mass-erase register sequence and waits for completion
