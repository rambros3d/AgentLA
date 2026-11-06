## MODIFIED Requirements

### Requirement: Autonomous Target Detection
The firmware SHALL automatically identify target microcontrollers across ARM Cortex-M (STM32, RP2040/RP2350, Puya PY32) and WCH RISC-V (CH32V003, CH32V203) by querying DBGMCU, IDCODE, or DMI registers.

#### Scenario: STM32F103 detection
- **WHEN** target detection is triggered via CLI `TARGET DETECT` or REST API `GET /api/v1/programmer/target`
- **THEN** the firmware reads DBGMCU IDCODE `0x410` and reports family `STM32F103 Medium Density` with flash size and SRAM boundaries

#### Scenario: CH32V003 detection
- **WHEN** target detection is triggered on a 1-wire target
- **THEN** the firmware initializes SWIO, verifies the QingKe debug module, and reports family `CH32V003`

#### Scenario: Raspberry Pi Pico 2 (RP2350) target detection
- **WHEN** target detection is triggered with a Raspberry Pi Pico 2 connected to the SWD pins
- **THEN** the firmware reads DPIDR and part number `0x00D` or IDCODE `0x4C013477`, successfully reporting `RP2350`
