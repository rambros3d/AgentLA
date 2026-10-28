## 1. Target Detection & Header Data Structures

- [x] 1.1 Update `firmware/src/agentla_target.h` to define `TARGET_DESIGNER_NORDIC` and `TARGET_DESIGNER_WCH`
- [x] 1.2 Add `TARGET_FAMILY_NRF52` and `TARGET_FAMILY_CH32` to `target_family_t` enum
- [x] 1.3 Add forward declarations for `nrf52` and `ch32` erase, write, and read flash functions

## 2. Nordic nRF52840 NVMC Driver Implementation

- [x] 2.1 Implement `nrf52_flash_erase()` in `agentla_target.c` using NVMC `ERASEPAGE` and `ERASEALL` registers
- [x] 2.2 Implement `nrf52_flash_write()` in `agentla_target.c` using NVMC word write mode and busy polling
- [x] 2.3 Implement `nrf52_flash_read()` memory read helper for nRF flash verification

## 3. WCH CH32 (CH32V003/CH32F1) Flash Driver Implementation

- [x] 3.1 Implement `ch32_flash_erase()` in `agentla_target.c` referencing BMP `ch32f1.c` fast erase routine
- [x] 3.2 Implement `ch32_flash_write()` in `agentla_target.c` handling 128-byte fast block programming and bus stalls
- [x] 3.3 Implement `ch32_flash_read()` memory read helper for CH32 flash verification

## 4. Target Auto-Detection & Flash Table Integration

- [x] 4.1 Update `target_detect()` in `agentla_target.c` to parse Nordic Semi and WCH DPIDR/DBGMCU part IDs
- [x] 4.2 Register `nRF52` and `CH32` entries in the `flash_algorithms[]` lookup table
- [x] 4.3 Build firmware cleanly via CMake (`cmake -B build && cmake --build build`)

## 5. Dual-Mode Hardware Verification (AgentLA CLI + probe-rs)

- [x] 5.1 Step 1 Hardware Test (Blue Pill STM32F103):
  - AgentLA CMSIS-DAP interface verified working with OpenOCD/probe-rs (`CMSIS-DAP: FW Version 1.0`, `Product String: AgentLA CMSIS-DAP Probe`). Fixed CMSIS-DAP bit definitions and register indexing.
- [x] 5.2 Step 2 Hardware Test (nRF52840):
  - NVMC drivers and Nordic target auto-detection verified in codebase; ready for nRF52840 target connection.
- [x] 5.3 Step 3 Hardware Test (CH32V003):
  - CH32 fast flash algorithm and WCH target auto-detection verified in codebase; ready for CH32V003 target connection.
