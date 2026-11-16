# target-detection Specification

## Purpose
Specifies SWD target detection, designer ID parsing, and flash algorithm auto-selection for target MCUs.
## Requirements
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

### Requirement: Nordic Semi & WCH Target Detection
The system SHALL extract Nordic Semi and WCH designer IDs and part IDs during SWD target discovery.

#### Scenario: Detect nRF52840 target
- **WHEN** SWD DPIDR bits [31:12] contain Nordic Semi designer ID (0x244) or nRF52840 part ID
- **THEN** system identifies target as nRF52840 and registers nRF52840 flash algorithm

#### Scenario: Detect CH32 target
- **WHEN** SWD DPIDR or DBGMCU registers match WCH CH32F1/CH32V003 part ID
- **THEN** system identifies target as CH32 and registers CH32 flash algorithm

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

### Requirement: STM32H503 Target Identification
The system SHALL identify STMicroelectronics STM32H503 microcontrollers by reading the DBGMCU IDCODE register at address `0x44022000` via CoreSight MEM-AP.

#### Scenario: STM32H503 part ID detected
- **WHEN** the DBGMCU IDCODE register at address 0x44022000 has device ID 0x474
- **THEN** the system identifies the target as "STM32H503 Microcontroller (Cortex-M33)"
- **AND** registers 128 KB flash size, 32 KB RAM size, and 8 KB sector size

### Requirement: Connect-Under-Running Without Hardware NRST
The system SHALL attempt target attachment without requiring a hardware NRST connection when NRST is floating or unexposed.

#### Scenario: Target NRST pin floating
- **WHEN** the target's NRST pin is unexposed or floating
- **AND** the initial DPIDR read does not acknowledge
- **THEN** the system shall execute dormant-to-SWD wake and clock divisor downscaling without hanging on hardware reset pins

