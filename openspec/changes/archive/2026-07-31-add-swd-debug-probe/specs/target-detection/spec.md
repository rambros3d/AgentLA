## ADDED Requirements

### Requirement: Target IDCODE Read
The system SHALL read the target's IDCODE via SWD during initialization.

#### Scenario: Read IDCODE
- **WHEN** swd_read_idcode() is called
- **THEN** system reads DPIDR register and returns 32-bit IDCODE

#### Scenario: No target connected
- **WHEN** swd_read_idcode() is called with no target
- **THEN** system returns error (no ACK from target)

### Requirement: Designer ID Extraction
The system SHALL extract the designer ID from the IDCODE.

#### Scenario: Extract STMicroelectronics ID
- **WHEN** IDCODE bits [31:12] contain 0x043x
- **THEN** system identifies target as STM32 family

#### Scenario: Extract Raspberry Pi ID
- **WHEN** IDCODE bits [31:12] contain 0x483x
- **THEN** system identifies target as RP2040/RP2350

### Requirement: Flash Algorithm Auto-Selection
The system SHALL automatically select the appropriate flash algorithm based on IDCODE.

#### Scenario: Auto-detect STM32F1
- **WHEN** target IDCODE matches STM32F1 pattern
- **THEN** system selects STM32F1 flash algorithm

#### Scenario: Auto-detect RP2040
- **WHEN** target IDCODE matches RP2040 pattern
- **THEN** system selects RP2040 flash algorithm

#### Scenario: Unknown target
- **WHEN** target IDCODE does not match any known pattern
- **THEN** system returns error (unsupported target)

### Requirement: Target Info Report
The system SHALL report detected target information to host.

#### Scenario: Report target family
- **WHEN** target is detected
- **THEN** system can report target family name via DAP_Info

#### Scenario: Report flash size
- **WHEN** target is detected
- **THEN** system can report flash size via vendor-specific command
