## MODIFIED Requirements

### Requirement: STM32F1 Flash Algorithm
The system SHALL support STM32F1 family flash programming using ADIv5 AP register access via SWD, with correct F1 flash controller semantics: page erase, halfword programming, and explicit error detection.

#### Scenario: Erase STM32F1 page
- **WHEN** `target_flash_erase()` is called with the STM32F1 algorithm for a sub-range of flash
- **THEN** each 1 KiB page SHALL be erased individually using `FLASH_CR PER (0x02)` followed by `FLASH_AR = page_addr` and `FLASH_CR PER|STRT (0x42)`
- **AND** the system SHALL NOT use the mass-erase sequence (`MER|STRT 0x44`) for partial-range erases

#### Scenario: Mass erase STM32F1
- **WHEN** a full-chip erase is requested
- **THEN** the system SHALL use `FLASH_CR MER (0x04)` followed by `FLASH_CR MER|STRT (0x44)` exactly once per bank

#### Scenario: Write STM32F1 flash with halfword accesses
- **WHEN** `target_flash_write()` is called with the STM32F1 algorithm
- **THEN** data SHALL be programmed as 16-bit halfwords with the AP CSW size configured to HALFWORD (bits[2:0] = 0b001)
- **AND** each halfword SHALL be pre-positioned in the DRW byte lane selected by the target address bit [1] (bits [15:0] for `addr%4==0`, bits [31:16] for `addr%4==2`) — the F1 AHB-AP does not right-align sub-word data, mirroring BMP `adiv5_pack_data()` `value << (8 * (dest & 2))` (corrected 2026-08-13; see `import-debugprobe-pio-swd`)
- **AND** 32-bit AP accesses SHALL NOT be used for flash programming data

#### Scenario: Flash error flags are detected and reported
- **WHEN** an erase or write operation completes
- **THEN** the system SHALL clear stale `EOP` in `FLASH_SR` before the operation
- **AND** after the operation SHALL check `FLASH_SR` for `PGERR (0x04)` and `WRPRTERR (0x10)`
- **AND** SHALL report failure (with the FLASH_SR value) instead of returning success when error flags are set

#### Scenario: Write loop aborts on first error
- **WHEN** a halfword write results in `PGERR` or `WRPRTERR`
- **THEN** the write loop SHALL stop immediately and report the error
- **AND** SHALL NOT report `OK` for data that was not programmed
