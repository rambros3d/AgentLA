## Context

AgentLA currently supports SWD debugging for ARM Cortex-M targets (STM32F103) using standard 2-wire SWCLK/SWDIO signals on PIO1. WCH microcontrollers use RISC-V cores with two distinct physical and protocol debug interfaces:
1. **CH32V003**: Uses 1-wire SDI (Single-Wire Debug Interface) on pin `PD1` (GPIO5 SWDIO), using pulse-width modulation for bit cells.
2. **CH32V203 / CH32V307**: Uses 2-wire RV-SWD (`SWCLK` on GPIO4, `SWDIO` on GPIO5) with RISC-V DTM / DMI (Debug Module Interface) instead of ARM ADIv5.

To add seamless CH32 target support without breaking existing ARM SWD features, we need a unified RISC-V debug module transport layer and dedicated PIO state machines.

## Goals / Non-Goals

**Goals:**
- Implement 1-wire WCH SDI PIO state machine (`agentla_wch_sdi.pio`) for CH32V003.
- Implement 2-wire RISC-V DTM/DMI protocol engine (`agentla_riscv_dmi.c`) for CH32V203 / CH32V307.
- Support core reset, halt, resume, register access, and memory read/write operations via DMI abstract commands.
- Implement WCH Flash PE Controller drivers supporting fast page erase and page buffer streaming writes.
- Add CLI commands (`CH32 DETECT`, `CH32 FLASH`, `CH32 ERASE`) and Python verification test scripts.

**Non-Goals:**
- Supporting 64-bit RISC-V chips (e.g. CH569 / RV64).
- GDB remote server integration (can be addressed in a future change).

## Decisions

### 1. Separate PIO Programs for 1-Wire SDI vs 2-Wire SWD
- **Choice**: Write a dedicated `agentla_wch_sdi.pio` program for CH32V003 on PIO1 rather than overloading `agentla_swd.pio`.
- **Rationale**: 1-wire SDI uses single-wire bidirectional pulse width encoding/decoding, which requires custom PIO delay counters and edge sampling distinct from 2-wire clock/data SWD.
- **Alternatives Considered**: Software bit-banging on GPIO5 (rejected due to jitter and CPU usage overhead at > 1 MHz speeds).

### 2. Unified RISC-V DMI Layer
- **Choice**: Implement a clean `riscv_dmi_read()` and `riscv_dmi_write()` interface abstraction layer that routes through either 1-wire SDI or 2-wire RV-SWD physical transports based on the active target protocol mode.
- **Rationale**: Standardizes RISC-V Debug Module operations (`DMCONTROL`, `DMSTATUS`, `ABSTRACTCS`, `COMMAND`, `DATA0`) across all WCH family devices.

### 3. Fast Flash Buffer Streaming Mode
- **Choice**: Utilize WCH `FLASH_MODEKEYR` (`0x40022024`) fast-page programming with `FTPG` and `BUF_LOAD` buffer streaming.
- **Rationale**: Standard word-by-word programming on WCH Flash is extremely slow (~10 ms per word). Fast buffer mode streams 64/128 bytes into SRAM buffers before committing in a single flash write cycle.

## Risks / Trade-offs

- **[Risk] 1-Wire Half-Duplex Turnaround Timing** → **Mitigation**: Configure PIO `pindirs` transition with explicit cycle delays to prevent bus contention on GPIO5.
- **[Risk] Flash Buffer Bus Stalls** → **Mitigation**: Implement dummy flash reads to clear bus arbitration delays during buffer load as mandated by WCH FPEC documentation.

## Migration Plan

- All new code is additive under `firmware/pio/agentla_wch_sdi.pio`, `firmware/src/agentla_ch32.c/.h`, and `firmware/src/agentla_riscv_dmi.c/.h`.
- Existing `SWD` commands remain completely unaffected for ARM targets.
