## Why

The `firmware/src/` directory has grown to 26 source files in a flat structure with no separation of concerns. Analyzer capture logic, programmer/debug code, adapter bus drivers, and shared infrastructure are all intermixed. This makes it harder to understand module boundaries, manage dependencies, and reason about which code belongs to which functional domain. The subdirectories `analyzer/`, `adapter/`, and `programmer/` already exist but are empty — the reorganization fills them.

## What Changes

- Move analyzer-specific files (`agentla_capture.c/h`, `agentla_decode.c/h`) into `firmware/src/analyzer/`
- Move adapter-specific files (`agentla_adapter.c/h`) into `firmware/src/adapter/`
- Move programmer/debug-specific files into `firmware/src/programmer/`:
  - `agentla_programmer.c/h` — UART bootloader protocol
  - `agentla_swd.c/h` — SWD engine
  - `agentla_tap_pio.c/h` — PIO state machine
  - `agentla_cmsis_dap.c/h` — CMSIS-DAP wire protocol
  - `agentla_usb_hid.c/h` — USB HID transport
  - `agentla_target.c/h` — Target detection + flash algorithms
  - `agentla_ch32.c/h` — CH32 1-wire SDI + flash via debug
  - `agentla_riscv_dmi.c/h` — RISC-V DMI register access
- Keep shared files in `firmware/src/` root:
  - `agentla_board_settings.h` — Pin assignments, constants
  - `agentla_structs.h` — Shared type definitions
  - `main.c` — Command router
  - `tusb_config.h` — USB stack configuration
- Update all `#include` paths across moved files
- Update `CMakeLists.txt` file glob patterns

No behavioral changes. No API changes. No new features. Pure structural refactor.

## Capabilities

### New Capabilities

_(None — this is a structural reorganization, not a feature change.)_

### Modified Capabilities

_(No spec-level behavior changes. Implementation file locations change, but the external command interface and firmware behavior remain identical.)_

## Impact

- **Code affected**: All 22 moved `.c`/`.h` files need `#include` path updates. `main.c` and `CMakeLists.txt` need path updates.
- **APIs**: None changed — all public function signatures remain identical.
- **Dependencies**: No new libraries. No removed libraries.
- **Build system**: `CMakeLists.txt` glob patterns must be updated from `firmware/src/*.c` to include subdirectories.
- **Risk**: Low — moving files and updating includes is mechanical. The build must compile cleanly after the move.
