## MODIFIED Requirements

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

### Requirement: Target Detection
The system SHALL detect ARM Cortex-M targets via JEP106 identity in DPIDR.

#### Scenario: Detect ARM Cortex-M3
- **WHEN** target_detect() reads DPIDR
- **THEN** dpidr_identity == 0x3B identifies ARM Cortex and target_is_connected() returns true
