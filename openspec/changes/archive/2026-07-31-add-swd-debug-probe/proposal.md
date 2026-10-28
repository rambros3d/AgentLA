## Why

The AgentLA firmware currently supports UART bootloader flashing (STM32, RP2350) but lacks debug capabilities. SWD (Serial Wire Debug) is the ARM standard for on-chip debugging and flash programming. Adding CMSIS-DAP compatible SWD support enables:

1. **Universal flash programming** — Works with all STM32 families without requiring BOOT0 pin manipulation
2. **Full debug support** — Halt, step, breakpoints, register access via OpenOCD/GDB
3. **Industry compatibility** — Works with standard tools (OpenOCD, pyOCD, Keil, IAR)
4. **Pin efficiency** — Uses existing GPIO 6-7 already allocated for SWD

The debugprobe reference implementation (MIT licensed) provides a proven PIO-based SWD engine that can be adapted for the RP2350.

## What Changes

- **Add PIO-based SWD engine** — Port debugprobe's `probe.pio` to PIO1 for bit-accurate SWD timing
- **Add CMSIS-DAP protocol layer** — USB HID interface compatible with OpenOCD/pyOCD
- **Add flash algorithms** — STM32F1, STM32F4, STM32G0, RP2040/RP2350 flash programming
- **Add target auto-detection** — Read IDCODE and automatically select flash algorithm
- **Add SWD commands** — `SWD INIT`, `SWD READ`, `SWD WRITE`, `SWD IDCODE`, `SWD RESET`
- **Keep UART bootloader as fallback** — Existing UART bootloader code preserved for recovery scenarios

## Capabilities

### New Capabilities

- `swd-engine`: PIO-based SWD bit-bang engine for GPIO 6-7. Handles SWCLK generation, SWDIO bidirectional control, and 1-cycle turnaround timing.
- `cmsis-dap`: USB HID implementation of CMSIS-DAP v2 protocol. Enables compatibility with OpenOCD, pyOCD, and GDB debug tools.
- `flash-algorithms`: Target-specific flash programming algorithms. Auto-detect target MCU family and apply correct erase/write/read sequences.
- `target-detection`: Auto-detect target MCU via SWD IDCODE. Extract designer ID and part ID to select appropriate flash algorithm.

### Modified Capabilities

- `programmer-mode`: Add SWD as primary debug interface alongside existing UART bootloader fallback

## Impact

**Files to Create:**
- `firmware/pio/agentla_swd.pio` — PIO program (from debugprobe)
- `firmware/src/agentla_swd.h` / `.c` — SWD implementation
- `firmware/src/agentla_cmsis_dap.h` / `.c` — CMSIS-DAP protocol
- `firmware/src/agentla_flash.h` / `.c` — Flash algorithm interface
- `firmware/src/agentla_flash_stm32f1.c` — STM32F1 flash
- `firmware/src/agentla_flash_stm32f4.c` — STM32F4 flash
- `firmware/src/agentla_flash_stm32g0.c` — STM32G0 flash
- `firmware/src/agentla_flash_rp2040.c` — RP2040/RP2350 flash

**Files to Modify:**
- `firmware/src/main.c` — USB HID setup, SWD command handlers
- `CMakeLists.txt` — Add new source files

**Hardware:**
- GPIO 6 (SWCLK) — PIO1, already allocated
- GPIO 7 (SWDIO) — PIO1, already allocated
- GPIO 3 (nRST) — Target reset control
- USB VID:0x1209 PID:0x414D (different from analyzer mode)

**Dependencies:**
- Pico SDK (already used)
- TinyUSB (already in SDK)
- debugprobe PIO program (MIT licensed, reference only)
