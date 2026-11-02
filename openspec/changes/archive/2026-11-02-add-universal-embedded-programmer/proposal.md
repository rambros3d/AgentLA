## Why

AgentLA can already *debug* any ARM Cortex-M through its CMSIS-DAP path, but its own probe-driven flash programming only covers the STM32F1 flash controller. Every additional MCU family (STM32F4/G0/H7, Nordic, WCH, …) currently requires a hand-written, per-family register algorithm, and locked/read-protected parts cannot be recovered at all. The goal is for AgentLA to become a **standalone, cross-vendor embedded programmer** — covering normal programming and locked-target recovery — without a host tool, while keeping the open CMSIS-DAP debug path intact.

## What Changes

- **Add universal target identification**: detect vendor, family, protection level (RDP/APPROTECT/etc.) and factory flash/RAM sizes instead of hard-coded density tables.
- **Add a generic Cortex-M flash-algorithm runner**: upload an official algorithm blob into target SRAM and execute `Init`/`EraseSector`/`ProgramPage`/`Verify`/`UnInit` via Cortex-M debug registers, replacing hand-written per-family flash controllers with one generic engine.
- **Add ARM debug mass-erase recovery**: recover RDP Level 1 parts via ADIv5 AP writes (mass erase even when reads are blocked).
- **Add UART bootloader recovery modules**: ST AN3155-family bootloaders (with BOOT0/nRST control on GPIO7) plus WCH bootloader support, for parts where debug mass-erase is unavailable or disabled.
- **Safety and confirmation discipline**: verify part ID and flash size before destructive operations; reuse `CONFIRM`; require explicit consent to run uploaded target code.
- **Retire direct per-family register algorithms** (starting with STM32F1) once the generic runner reproduces the existing F1 5/5 hardware suite.
- **Keep CMSIS-DAP (debug) and RVSWD/SWIO paths untouched.**

## Capabilities

### New Capabilities
- `universal-target-detection`: Identify vendor/family/protection level and factory geometry from SWD, with per-family strategies for STM32, Nordic, WCH.
- `generic-cortexm-algo-runner`: Execute uploaded flash algorithm blobs in target RAM via Cortex-M debug registers, replacing per-family register code.
- `debug-mass-erase-recovery`: Recover protected ARM targets by mass-erase through ADIv5 AP writes.
- `bootloader-recovery`: Vendor UART-bootloader based flashing and recovery (ST AN3155 family, WCH), using default BOOT0 GPIO2 (push-pull) + GPIO6 nRST control, board-configurable.
- `programmer-safety`: Part ID/size verification before erase, confirmation for destructive ops, explicit consent for running target-side code.

### Modified Capabilities
- `flash-algorithms`: Flash-programming requirements change from "hand-coded per-family direct register sequences" to "generic algorithm execution as the primary implementation", with direct algorithms retained only as a transitional fallback.
- `programmer-control`: Adds BOOT0 pin control (GPIO7) and recovery-oriented reset sequencing alongside the existing nRST/power commands.
- `programmer-uart`: Expands bootloader requirement coverage to define BOOT0/nRST recovery orchestration rather than raw protocol only.

## Impact

- Firmware: `firmware/src/programmer/target.c` (detection, mass-erase), new `firmware/src/programmer/algo_runner.c` (+ `.h`), `bootloader_recovery.c`, `main.c` command surface, `board_settings.h` (GPIO7 BOOT0); PIO/USB/CMSIS-DAP and RVSWD paths are not affected.
- Tooling: host-side converter from Keil/CMSIS `.flm` to AgentLA blob format added to `sdk/`.
- Hardware: a dedicated BOOT0 GPIO must be wired for bootloader recovery; tests in `test/` extended for mass-erase, algo-runner parity, and bootloader recovery using Blue Pill as reference.
- Specs: new specs under `openspec/specs/{universal-target-detection,generic-cortexm-algo-runner,debug-mass-erase-recovery,bootloader-recovery,programmer-safety}/`; modified specs for `flash-algorithms`, `programmer-control`, `programmer-uart`.
- Risk: executing target-side code requires trust/consent; part mis-detection could brick a target, mitigated by mandatory pre-erase identity checks.