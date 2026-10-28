## Why

To guarantee 100% hardware reliability of the AgentLA RP2350 probe with STM32 Cortex-M3 microcontrollers, we need a comprehensive, automated 5-tier hardware verification suite combined with low-level hardware SWD signal diagnostics. This suite systematically tests protocol framing, Cortex-M3 core debugging, AHB memory access, STM32F1 flash programming algorithms (FPEC), real-time logic analyzer signal validation, and automatic SWD WAIT retry handling against a physical Blue Pill target.

## What Changes

- Add automated **5-tier hardware verification test script** (`test/test_stm32f1_hardware_suite.py`) covering:
  - **Tier 1**: Transport & SWD protocol framing (DPIDR `0x1BA01477`, `DP_CTRLSTAT` power-up, AP_IDR `0x04770001`, and frequency scaling from 100 kHz to 5 MHz).
  - **Tier 2**: Cortex-M3 CPU control (Halt/Resume via DHCSR, core register reading/writing, single-step execution, and reset vector catch).
  - **Tier 3**: AHB memory bus & chip identification (SRAM 8/16/32-bit read/write, DBGMCU part ID `0x410`, Flash Size register `0x1FFFF7E0`, and 96-bit chip Unique ID).
  - **Tier 4**: FPEC Flash controller (Key unlock `0x45670123`/`0xCDEF89AB`, 1 KiB page erase, 16-bit CSW halfword programming, and latched error flag detection).
  - **Tier 5**: Application deployment & signal verification (Blink binary payload flash, byte-for-byte readback verification, AIRCR system reset, and logic analyzer sampling of PC13 LED on GPIO18).
- Add low-level **hardware SWD signal debugging & WAIT recovery**:
  - Auto-clock frequency fallback (sweep 1000 kHz → 500 kHz → 100 kHz) on persistent `SWD_ACK_WAIT` (`011`).
  - Signal integrity diagnostic capture on SWCLK/SWDIO pins via AgentLA 100MHz logic analyzer.
  - Interactive CDC serial CLI diagnostic commands **`STM32 TEST`** and **`SWD DIAG`** in `firmware/src/main.c`.
- Update `docs/programmer.md` with complete 5-tier hardware test procedures, signal capture debugging steps, and expected target responses.

## Capabilities

### New Capabilities
- `stm32f1-hardware-verification`: Automated 5-tier verification suite, low-level SWD signal debugging, and CDC diagnostic commands to test AgentLA RP2350 probe capabilities against STM32F1 (Blue Pill) hardware.

### Modified Capabilities
- None.

## Impact

- **Test Infrastructure**: Adds `test/test_stm32f1_hardware_suite.py` and diagnostic scripts.
- **Firmware**: Adds `cmd_stm32_test`, `cmd_swd_diag`, and clock fallback in `firmware/src/main.c` & `firmware/src/programmer/swd.c`.
- **Documentation**: Updates `docs/programmer.md`.
