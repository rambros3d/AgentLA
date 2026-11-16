## ADDED Requirements

### Requirement: STM32H5 Flash Algorithm
The system SHALL support STM32H5 family flash programming using ADIv5 AHB-AP register access via SWD with 128-bit quad-word programming, 8 KB sector erase, and 1 KiB TAR auto-increment boundary chunking.

#### Scenario: Erase STM32H5 sector
- **WHEN** flash erase is requested for STM32H5 sector N
- **THEN** sector N is erased using FLASH_NSCR SER and SNB bits and verified blank

#### Scenario: Write STM32H5 flash in quad-words
- **WHEN** flash write is requested for STM32H5 memory
- **THEN** data is programmed in 128-bit (16-byte) quad-words using FLASH_NSCR PG bit and verified by readback

#### Scenario: Flash unlock and lock control
- **WHEN** STM32H5 flash unlock is called
- **THEN** FLASH_NSKEYR is written with KEY1 (0x45670123) and KEY2 (0xCDEF89AB) and LOCK bit is verified clear

#### Scenario: 1 KiB TAR boundary crossing
- **WHEN** contiguous memory transfer crosses a 1024-byte boundary
- **THEN** TAR is explicitly reloaded at the boundary chunk to avoid CoreSight 10-bit address wrap
