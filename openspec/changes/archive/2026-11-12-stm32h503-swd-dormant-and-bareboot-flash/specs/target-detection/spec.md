## ADDED Requirements

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
