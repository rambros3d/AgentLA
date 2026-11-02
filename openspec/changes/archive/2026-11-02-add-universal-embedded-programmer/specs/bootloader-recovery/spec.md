## ADDED Requirements

### Requirement: Bootloader-based recovery
The system SHALL support flashing and recovering parts through vendor UART bootloaders when debug recovery is unavailable.

#### Scenario: ST part recovery via AN3155
- **WHEN** a locked ST part that exposes the AN3155 bootloader is placed in bootloader mode
- **THEN** the system performs sync, erase, program, verify, and reset to application

#### Scenario: Bootloader unavailable
- **WHEN** the target does not respond to bootloader sync
- **THEN** the system returns `ERR BOOTLOADER NOT FOUND`

### Requirement: BOOT0 and reset orchestration
The system SHALL control BOOT0 (a dedicated GPIO selected in board configuration) and nRST (GPIO6) to enter and exit bootloader mode without external wiring beyond BOOT0.

#### Scenario: Enter bootloader mode
- **WHEN** bootloader recovery is requested
- **THEN** the system asserts BOOT0 high, pulses nRST low, waits, and then begins sync

#### Scenario: Exit bootloader mode
- **WHEN** bootloader recovery completes (success or failure)
- **THEN** the system releases BOOT0, pulses nRST, and restores normal operation

### Requirement: WCH bootloader support
The system SHALL support WCH bootloader-based flashing and recovery as a distinct vendor module.

#### Scenario: WCH sync
- **WHEN** a CH32 target exposes the WCH bootloader
- **THEN** the system performs WCH-specific sync, program, and verify flows

### Requirement: Bootloader documentation of coverage
The system SHALL document which target families and variants expose a bootloader suitable for recovery.

#### Scenario: Coverage documented
- **WHEN** a user consults the programming docs
- **THEN** the subset of ST/Nordic/WCH parts with recovery-reachable bootloaders is listed