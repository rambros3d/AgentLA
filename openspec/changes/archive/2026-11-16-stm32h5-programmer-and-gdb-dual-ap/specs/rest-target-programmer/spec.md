## ADDED Requirements

### Requirement: Non-Intrusive Live Memory Read
The HTTP server SHALL provide `GET /api/v1/programmer/read` to read target memory regions non-intrusively via the appropriate Access Port without requiring core execution to be halted.

#### Scenario: Reading flash memory on STM32H5 target
- **WHEN** client sends `GET /api/v1/programmer/read?addr=134217728` (`0x08000000`) for an STM32H5 target
- **THEN** server selects AP 1 (`AHB-AP`) and reads 16 bytes of flash memory via TAR/DRW without requiring target halt, returning HTTP 200 with the hex data payload.

### Requirement: STM32H5 DBGMCU Low-Power Debug Un-gating
The target programmer SHALL configure `DBGMCU_CONFIG` on STM32H5 targets upon SWD attachment to prevent `WFI` / `WFE` instructions from disabling debug clocking.

#### Scenario: Flash programming on STM32H5 running low-power firmware
- **WHEN** client issues `POST /api/v1/programmer/flash` or `POST /api/v1/programmer/erase` to an STM32H5 executing `WFI`
- **THEN** programmer writes `DBG_STOP | DBG_STANDBY` to `DBGMCU_CONFIG` (`0x44024004`), successfully halts the core via DHCSR on AP 1, and completes flash programming.
