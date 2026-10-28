## 1. Move and Rename Analyzer Files

- [x] 1.1 Move `firmware/src/agentla_capture.c` → `firmware/src/analyzer/capture.c`
- [x] 1.2 Move `firmware/src/agentla_capture.h` → `firmware/src/analyzer/capture.h`
- [x] 1.3 Move `firmware/src/agentla_decode.c` → `firmware/src/analyzer/decode.c`
- [x] 1.4 Move `firmware/src/agentla_decode.h` → `firmware/src/analyzer/decode.h`

## 2. Move and Rename Adapter Files

- [x] 2.1 Move `firmware/src/agentla_adapter.c` → `firmware/src/adapter/adapter.c`
- [x] 2.2 Move `firmware/src/agentla_adapter.h` → `firmware/src/adapter/adapter.h`

## 3. Move and Rename Programmer Files

- [x] 3.1 Move `firmware/src/agentla_programmer.c/h` → `firmware/src/programmer/programmer.c/h`
- [x] 3.2 Move `firmware/src/agentla_swd.c/h` → `firmware/src/programmer/swd.c/h`
- [x] 3.3 Move `firmware/src/agentla_tap_pio.c/h` → `firmware/src/programmer/tap_pio.c/h`
- [x] 3.4 Move `firmware/src/agentla_cmsis_dap.c/h` → `firmware/src/programmer/cmsis_dap.c/h`
- [x] 3.5 Move `firmware/src/agentla_usb_hid.c/h` → `firmware/src/programmer/usb_hid.c/h`
- [x] 3.6 Move `firmware/src/agentla_target.c/h` → `firmware/src/programmer/target.c/h`
- [x] 3.7 Move `firmware/src/agentla_ch32.c/h` → `firmware/src/programmer/ch32.c/h`
- [x] 3.8 Move `firmware/src/agentla_riscv_dmi.c/h` → `firmware/src/programmer/riscv_dmi.c/h`

## 4. Update CMakeLists.txt

- [x] 4.1 Update `FILE(GLOB CSources firmware/src/*.c)` to recursively glob subdirectories
- [x] 4.2 Verify PIO header generation paths still resolve correctly

## 5. Rename Shared Files and Update Includes

- [x] 5.1 Rename `firmware/src/agentla_board_settings.h` → `firmware/src/board_settings.h`
- [x] 5.2 Rename `firmware/src/agentla_structs.h` → `firmware/src/structs.h`
- [x] 5.3 Update all `#include` statements throughout codebase
- [x] 5.4 Remove "agentla_" prefix from all moved files

## 6. Verify Build

- [x] 6.1 Run `cmake -S . -B build` and verify configuration succeeds
- [x] 6.2 Run `cmake --build build` and verify compilation succeeds with no errors
- [x] 6.3 Verify no `#include` resolution warnings in build output
