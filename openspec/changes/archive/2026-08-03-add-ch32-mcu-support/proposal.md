## Why

AgentLA currently supports SWD programming and logic analysis for ARM Cortex-M microcontrollers (such as STM32F103). To expand AgentLA into a versatile multi-architecture programmer, we need support for popular WCH RISC-V microcontrollers, including the low-cost 1-wire SDI CH32V003 (QingKe V2A core) and 2-wire RV-SWD CH32V203 / CH32V307 (QingKe V4 core) devices.

## What Changes

- Add **CH32V003 1-Wire SDI** protocol implementation using RP2350 PIO state machine encoding/decoding on `GPIO5 (SWDIO)`.
- Add **2-Wire RISC-V SWD (RV-SWD)** protocol support to interface with RISC-V Debug Transport Module (DTM) and Debug Module Interface (DMI) registers on `GPIO4 (SWCLK)` and `GPIO5 (SWDIO)`.
- Add **RISC-V Debug Module Driver** handling core halt/resume, register access, and memory bus read/write operations via DMI abstract commands.
- Add **WCH Flash Memory Controllers** supporting fast page erase (64B / 128B / 1KB / 2KB) and RAM buffer fast page write mode.
- Add CLI commands (`CH32 DETECT`, `CH32 FLASH`, `CH32 ERASE`) and Python verification test scripts.

## Capabilities

### New Capabilities
- `ch32-sdi-protocol`: 1-wire pulse width encoding/decoding on RP2350 PIO for CH32V003 single-wire debug.
- `riscv-debug-module`: RISC-V Debug Transport Module (DTM) and Debug Module Interface (DMI) protocol engine over 2-wire SWD and 1-wire SDI.
- `ch32-flash-programmer`: Target probe, electronic signature (ESIG) detection, memory map identification, and fast page flash programming for CH32V003 and CH32V203/V307 MCUs.

### Modified Capabilities
- `swd-programmer`: Expanded transport protocol layer to support both ARM ADIv5 and RISC-V DTM packets.

## Impact

- Firmware source code: New PIO file `firmware/pio/agentla_wch_sdi.pio`, driver headers `firmware/src/agentla_ch32.h` and `agentla_riscv_dmi.h`, and implementation files `firmware/src/agentla_ch32.c` and `agentla_riscv_dmi.c`.
- CLI commands: New serial commands added to `firmware/src/main.c`.
- Test suite: New python integration test scripts `test/test_ch32v003.py` and `test/test_ch32v203.py`.
