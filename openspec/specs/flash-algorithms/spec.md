# flash-algorithms

## Purpose
Defines flash programming algorithms and interfaces, providing both direct register algorithms and generic RAM-resident CMSIS algorithms for embedded targets.

## Requirements

### Requirement: STM32F1 Flash Algorithm
The system SHALL support STM32F1 family flash programming using ADIv5 AP register access via SWD.

#### Scenario: Erase STM32F1 sector
- **WHEN** flash_erase() is called with STM32F1 algorithm
- **THEN** specified sector is erased using FLASH_CR ER bit and FLASH_AR address register

#### Scenario: Write STM32F1 flash
- **WHEN** flash_write() is called with STM32F1 algorithm
- **THEN** data is written using FLASH_CR PG bit with halfword programming

#### Scenario: Flash unlock verification
- **WHEN** stm32f1_unlock_flash() is called
- **THEN** FLASH_KEYR is written with KEY1 (0x45670123) then KEY2 (0xCDEF89AB) and LOCK bit is verified clear

#### Scenario: BSY wait uses correct ADIv5 read pattern
- **WHEN** stm32f1_wait_flash_bsy() polls FLASH_SR
- **THEN** each SR read SHALL use ap_read() helper for ADIv5 dummy+actual read pattern

#### Scenario: Flash error flags use correct mask
- **WHEN** FLASH_SR is checked after erase or write
- **THEN** error mask SHALL be 0x0C (WRPERR|PGERR) and clear mask SHALL be 0x0E (EOP|WRPERR|PGERR)

### Requirement: Generic algorithm as primary flash implementation
The system SHALL support executing uploaded official flash algorithms in target RAM as the primary programming path, with direct register algorithms retained only as a transitional fallback.

#### Scenario: Generic path available
- **WHEN** a valid algorithm blob and matching metadata are supplied for a connected target
- **THEN** the system programs flash through the generic algorithm path and reports `OK FLASH DONE`

#### Scenario: Fallback retained during transition
- **WHEN** generic algorithm support is disabled or unavailable for a known family
- **THEN** the system falls back to the existing direct register algorithm without changing observable results
