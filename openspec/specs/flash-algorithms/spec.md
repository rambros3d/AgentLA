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
The system SHALL support a 3-tier flash resolution cascade: executing direct register engines or ROM-embedded RAM loaders (Tier 1) for catalog targets, executing LittleFS-resident FLM blobs (Tier 2) under `/system/algo/<name>.bin`, and streaming host-supplied FLM payloads (Tier 3) into target RAM.

#### Scenario: Direct engine execution for catalog target
- **WHEN** a recognized catalog target is programmed and no override is requested
- **THEN** the system executes the assigned direct register engine or ROM-embedded RAM loader (Tier 1) without requiring LittleFS access.

#### Scenario: LittleFS FLM execution for extended targets
- **WHEN** an uncataloged target has a matching `.bin` algorithm installed in LittleFS `/system/algo/`
- **THEN** the system reads the 44-byte `AlgoBlobHeader`, streams the executable into target SRAM, and executes `ProgramPage` (Tier 2).

#### Scenario: Host FLM dynamic streaming
- **WHEN** a target is neither in Tier 1 nor Tier 2, but the host/MCP client streams an FLM blob payload
- **THEN** the system loads the provided RAM algorithm and completes programming (Tier 3).
