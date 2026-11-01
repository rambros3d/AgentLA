# Proposal: Import RVSWD PIO Transport & CH32V203 Hardware Debugging

## Why

Following the successful migration of ARM SWD to Raspberry Pi's vendored `debugprobe` PIO engine (which achieved 100% 12/12 passing hardware tests), AgentLA's RVSWD (2-wire RISC-V debug protocol for WCH QingKe V4 cores like CH32V203 / CH32V307) needs the same level of architectural encapsulation and reliability. 

AgentLA's existing RVSWD implementation contains critical flaws:
- Destructive PIO instruction memory clearing (`pio_clear_instruction_memory(pio1)`), wiping out co-existing state machines.
- Unbounded FIFO wait loops that can hang the single-threaded probe if target pins are disconnected.
- Scattered logic between `rvswd.c`, `ch32.c`, `riscv_dmi.c`, and `rvswd.pio`.

By vendoring and isolating the proven PIO RVSWD driver from `references/RVSWD_pico` into `firmware/src/programmer/rvswd/` and dynamically sharing `PIO0 SM0` with ARM SWD, we achieve reference-grade performance and protocol stability for CH32 RISC-V targets while keeping `PIO1` 100% available for SWIO (1-wire CH32V003) and UART bridges.

## What Changes

- **Vendor & Isolate RVSWD PIO Engine**: Create `firmware/src/programmer/rvswd/` containing vendored `rvswd.pio` (from `references/RVSWD_pico/RVSWD_full.pio`), `rvswd_pio_transport.c`, and `rvswd_pio_transport.h`.
- **Dynamic PIO0 SM0 Allocation**: Share `PIO0 SM0` and GPIO4/5 dynamically between ARM SWD (`probe.pio`) and RVSWD (`rvswd.pio`). When switching targets, `swd_deinit()` unloads `probe.pio` via `pio_remove_program()`, allowing `rvswd_init()` to load `rvswd.pio` into `PIO0` (25 instructions $\le$ 32 instruction limit).
- **Non-blocking Bounded FIFOs**: Wrap all RVSWD PIO FIFO puts and gets with 1ms timeouts (`rvswd_put_bounded`) to ensure disconnected targets never freeze probe execution.
- **Wire Frame & Line Reset**: Implement 300-cycle PIO line reset loop, MSB-first left-aligned data transfer, 9-bit header generation with even parity, 5-bit turnaround (`0b10101`), 32-bit payload + parity, 5-bit trailer (`0b10111`), and clean start/stop states.
- **RISC-V DMI & DM Debug Control**: Connect `riscv_dmi_read()` / `riscv_dmi_write()` to RVSWD with auto-retry (up to 5 retries). Implement `ch32v20x_halt_microprocessor()`, `ch32v20x_resume_microprocessor()`, and CPU register read/write (`COMMAND` / `DATA0` abstract command interface).
- **ARM SWD BluePill Regression Verification**: Run the full BluePill ARM SWD test suite (`test_swd_hardware.py` 12/12 pass, `test_stm32f1_hardware_suite.py` 5/5 pass) FIRST after implementation to guarantee zero regressions to ARM SWD.
- **CH32V203 Hardware Verification Suite**: Build and execute end-to-end hardware verification on a CH32V203 dev board:
  1. Read target (`CH32 DETECT mcu=ch32v203` $\rightarrow$ DMSTATUS link up, chip_id readback).
  2. Flash target (erase page/full, program firmware payload, byte-perfect verify readback).
  3. Debug target (halt CPU, single-step instruction, resume CPU, read/write core registers R0-R15 / PC).

## Definition of Done

This plan is considered **COMPLETE** when and only when:
1. **BluePill ARM SWD Regression Test**: 100% pass (12/12 hardware SWD tests, 5/5 tier flash suite).
2. **CH32V203 Hardware Verification**: Confirmed working end-to-end on live CH32V203 hardware:
   - Target detect & ID readback SUCCESS
   - Flash erase, program, and verification SUCCESS
   - CPU halt, resume, step, and register mutation SUCCESS

## Impact

- **Code affected**: New directory `firmware/src/programmer/rvswd/`, `firmware/src/programmer/rvswd.c` (replaced/refactored), `firmware/src/programmer/rvswd.h`, `firmware/src/programmer/ch32.c`, `firmware/src/programmer/riscv_dmi.c`, `firmware/pio/rvswd.pio` (moved to driver folder).
- **PIO Allocation**: `PIO0 SM0` shared dynamically between ARM SWD (`probe.pio`, 26 insts) and RVSWD (`rvswd.pio`, 25 insts). `PIO1` remains 100% free for `swio.pio` and UART bridges.
- **Hardware Targets**: STM32F103 BluePill (ARM SWD regression sweep), CH32V203 Dev Board (RISC-V RVSWD primary target).
