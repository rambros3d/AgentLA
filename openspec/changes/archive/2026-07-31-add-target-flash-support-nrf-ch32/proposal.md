## Why

AgentLA currently supports SWD debugging and target flash programming for STM32F1 and RP2040/RP2350 target MCUs, but lacks native target flash algorithms for popular Nordic Semi (nRF52840) and WCH (CH32V003/CH32F1) microcontrollers. Adding native standalone flash programming for nRF52840 and CH32 MCUs—referencing proven target drivers from Black Magic Probe (BMP)—will expand AgentLA's multi-target hardware verification and autonomous AI agent debugging capabilities.

Hardware verification will perform **dual-mode testing** (AgentLA Standalone CLI + `probe-rs` over CMSIS-DAP) following a phased hardware test order:
1. **STM32F103 (Blue Pill)** - Baseline SWD & Flash verification (CLI & `probe-rs run --chip STM32F103C8T6`)
2. **nRF52840** - Nordic NVMC flash programming & APPROTECT handling (CLI & `probe-rs run --chip nRF52840_xxAA`)
3. **CH32V003** - WCH RISC-V 1-wire / SWD target detection and flash algorithm (CLI & `probe-rs` verification)

## What Changes

- Add target detection for Nordic Semi (`nRF52840`) and WCH (`CH32V003`/`CH32F1`) MCU families using DPIDR designer ID and part ID decoding.
- Port native flash programming algorithms from Black Magic Probe reference C code:
  - **nRF52840**: Implement Nordic NVMC (Non-Volatile Memory Controller) page erase, word write, and APPROTECT unlock sequences over SWD.
  - **CH32 (CH32V003 / CH32F1)**: Implement WCH fast-mode page erase and 128-byte block write algorithms.
- Register nRF52840 and CH32 flash algorithms in AgentLA's global target algorithm table (`agentla_target.c`).
- Update CLI commands (`SWD TEST`, `FLASH ERASE`, `FLASH WRITE`, `FLASH READ`) and CMSIS-DAP interface for dual-mode verification with `probe-rs`.

## Capabilities

### New Capabilities
- None (Extending existing target-detection and flash-algorithms capabilities).

### Modified Capabilities
- `flash-algorithms`: Add nRF52840 NVMC flash programming routines (page erase, word write) and CH32 fast flash programming routines (page erase, block write).
- `target-detection`: Add Nordic Semi (nRF52840) and WCH (CH32V003/CH32F1) designer IDs and part IDs to SWD auto-detection engine.

## Impact

- `firmware/src/agentla_target.h`: New target designer IDs (`TARGET_DESIGNER_NORDIC`, `TARGET_DESIGNER_WCH`) and family enums (`TARGET_FAMILY_NRF52`, `TARGET_FAMILY_CH32`).
- `firmware/src/agentla_target.c`: Target detection logic and NVMC / CH32 flash driver functions.
- Hardware verification targets: Blue Pill (STM32F103), nRF52840 Dongle/DK, CH32V003 Dev Board using both AgentLA CLI and host `probe-rs`.
