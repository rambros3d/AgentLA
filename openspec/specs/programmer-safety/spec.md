# programmer-safety

## Purpose
Enforces safety verification, geometry validation, confirmation tokens, and informative banners before destructive flash, erase, or bootloader operations are permitted on any connected target.

## Requirements

### Requirement: Pre-destructive identity verification
The system SHALL refuse erase, program, or recovery operations unless target identity and geometry are successfully established.

#### Scenario: Unidentified target
- **WHEN** an erase or program command is issued without a successful prior identify
- **THEN** the system returns `ERR TARGET NOT IDENTIFIED`

#### Scenario: Geometry mismatch
- **WHEN** the supplied address or size exceeds reported flash geometry
- **THEN** the system returns `ERR RANGE OUT OF FLASH`

### Requirement: Explicit confirmation for destructive operations
The system SHALL require explicit user confirmation for mass erase and bootloader recovery operations.

#### Scenario: Mass erase without confirmation
- **WHEN** a mass-erase command is issued without confirmation
- **THEN** the system returns `ERR CONFIRM REQUIRED` and performs no action

### Requirement: Informative destructive banners
The system SHALL report the part name, byte count, and protection level before performing destructive operations.

#### Scenario: Pre-erase banner
- **WHEN** a destructive operation is authorized
- **THEN** the system emits a banner identifying part, bytes, and protection level before proceeding
