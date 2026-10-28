## MODIFIED Requirements

### Requirement: STM32F1 part identification
The system SHALL identify STM32F1 density variants from the DBGMCU_IDCODE part ID so that the Blue Pill and similar boards are reported correctly with matching flash/RAM geometry.

#### Scenario: Identify STM32F1 medium density (Blue Pill)
- **WHEN** `target_detect()` reads DBGMCU_IDCODE part ID `0x410` (STM32F103C8, medium density)
- **THEN** the system SHALL report `target=STM32F103` with 64 KiB flash and 20 KiB RAM
- **AND** SHALL select the STM32F1 flash algorithm

#### Scenario: Identify other STM32F1 densities
- **WHEN** `target_detect()` reads part ID `0x412` (low density), `0x414` (high density), `0x418` (connectivity line), or `0x430` (XL density)
- **THEN** the system SHALL report the STM32F1 family with flash/RAM geometry matching that density
- **AND** SHALL select the STM32F1 flash algorithm

#### Scenario: No fallback to generic Cortex-M3 for known F1 parts
- **WHEN** `target_detect()` reads any of the known STM32F1 part IDs above
- **THEN** the system SHALL NOT fall through to the generic "STM32 (Cortex-M3)" default
