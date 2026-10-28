## Context

AgentLA currently provides SWD target detection and flash algorithm implementations for STM32F1 and RP2040/RP2350 target MCUs in `firmware/src/agentla_target.c`. To expand multi-target support for developers and AI agents, we are adding native flash programming for Nordic Semi nRF52840 and WCH CH32 (CH32V003 / CH32F1) microcontrollers, porting algorithms directly from Black Magic Probe (`references/blackmagic/src/target`).

Hardware verification will perform **dual-mode validation** (AgentLA Standalone CLI + `probe-rs` over CMSIS-DAP) across three distinct phases:
1. **STM32F103 (Blue Pill)**: Baseline verification of SWD commands via AgentLA CLI and `probe-rs run --chip STM32F103C8T6`.
2. **nRF52840 (Nordic Semiconductor)**: Verification of NVMC page erase, word write, and APPROTECT detection via AgentLA CLI and `probe-rs run --chip nRF52840_xxAA`.
3. **CH32V003 (WCH Microcontroller)**: Verification of CH32 fast-mode page erase and 128-byte block write algorithms via AgentLA CLI and `probe-rs`.

## Goals / Non-Goals

**Goals:**
- Implement target auto-detection for Nordic Semi (nRF52840) and WCH (CH32V003 / CH32F1) designer IDs and part IDs.
- Port native C flash algorithms from Black Magic Probe (`nrf51.c` and `ch32f1.c`) into AgentLA's `agentla_target.c`.
- Register `nRF52840` and `CH32` in AgentLA's `flash_algorithms[]` table.
- Perform dual-mode hardware flash verification (AgentLA CLI + host `probe-rs`) sequentially against Blue Pill, nRF52840, and CH32V003 target boards.

**Non-Goals:**
- **Embedded GDB Server**: No embedded GDB RSP server will be added. AgentLA strictly relies on CMSIS-DAP (for `probe-rs` / OpenOCD) and structured CDC CLI (for LLM agents).
- **JTAG Scan Chain**: AgentLA remains focused on SWD / 1-wire / UART interfaces.

## Decisions

### Decision 1: Direct Driver Porting from Black Magic Probe C Code
- **Choice**: Adapt register definitions and programming flows directly from `references/blackmagic/src/target/nrf51.c` and `references/blackmagic/src/target/ch32f1.c`.
- **Rationale**: BMP drivers are well-tested, battle-proven C implementations that run directly on bare-metal Cortex-M probes without relying on external host middleware.

### Decision 2: NVMC Memory Controller Interface for nRF52840
- **Choice**: Use Nordic's NVMC registers (`NVMC_READY`, `NVMC_CONFIG`, `NVMC_ERASEPAGE`, `NVMC_ERASEALL`) accessed via SWD AP memory transfers (`swd_write_ap`/`swd_read_ap`).
- **Rationale**: NVMC is the standardized flash controller across all nRF51/nRF52/nRF53 devices.

### Decision 3: Dual-Mode Testing Strategy (AgentLA CLI + probe-rs)
- **Choice**: Verify every target board with both AgentLA CLI commands (`SWD TEST`, `FLASH ERASE`, `FLASH WRITE`) AND host `probe-rs` CLI tool (`probe-rs run`).
- **Rationale**: Ensures both AgentLA's native standalone firmware flasher and CMSIS-DAP interface for host Rust tools are 100% functional.

### Decision 4: Phased Hardware Verification Order
- **Choice**: Step 1: Blue Pill (STM32F103) sanity check -> Step 2: nRF52840 testing -> Step 3: CH32V003 testing.
- **Rationale**: Verifying the baseline STM32F1 SWD engine first ensures no regression before testing newly ported NVMC and CH32 drivers.

## Risks / Trade-offs

- **[Risk] nRF APPROTECT Lockout**: If nRF52840 has APPROTECT enabled, normal SWD AP memory access fails.
  - *Mitigation*: Add CTRL-AP check in `target_detect()` and report APPROTECT status to host CLI and `probe-rs`.
- **[Risk] CH32 SWD Bus Stalls**: CH32 fast flash mode can stall the SWD AP bus during write buffer commits.
  - *Mitigation*: Include dummy read cycles and busy polling matching BMP's `ch32f1.c` workaround.

## Migration Plan

1. Update `agentla_target.h` header with enums and structs.
2. Implement driver functions in `agentla_target.c`.
3. Build firmware with CMake/Pico SDK (`cmake -B build && cmake --build build`).
4. Flash `AgentLA.uf2` to RP2040/RP2350 hardware.
5. Perform dual-mode hardware test suite (AgentLA CLI + `probe-rs`) across Blue Pill, nRF52840, and CH32V003.
