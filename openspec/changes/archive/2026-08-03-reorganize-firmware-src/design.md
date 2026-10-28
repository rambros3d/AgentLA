## Context

The `firmware/src/` directory currently contains 26 source files in a flat layout. The codebase serves three functional domains (analyzer, adapter, programmer) that share some infrastructure but have distinct concerns. Empty subdirectories `analyzer/`, `adapter/`, and `programmer/` already exist, signaling the intended structure was never completed.

Current flat layout:
```
firmware/src/
├── agentla_board_settings.h  (shared)
├── agentla_structs.h         (shared)
├── main.c                    (router)
├── tusb_config.h             (shared)
├── agentla_capture.c/h       (analyzer)
├── agentla_decode.c/h        (analyzer)
├── agentla_adapter.c/h       (adapter)
├── agentla_programmer.c/h    (programmer)
├── agentla_swd.c/h           (programmer)
├── agentla_tap_pio.c/h       (programmer)
├── agentla_cmsis_dap.c/h     (programmer)
├── agentla_usb_hid.c/h       (programmer)
├── agentla_target.c/h        (programmer)
├── agentla_ch32.c/h          (programmer)
└── agentla_riscv_dmi.c/h     (programmer)
```

## Goals / Non-Goals

**Goals:**
- Organize files into domain-specific subdirectories
- Make module boundaries explicit through directory structure
- Maintain identical build output (same binary, same behavior)
- Keep shared infrastructure accessible to all modules

**Non-Goals:**
- Changing any public API or function signature
- Adding new features or capabilities
- Refactoring internal logic within any file
- Changing the PIO programs or build flags

## Decisions

### 1. Shared files stay in root, not in a `common/` subdirectory

**Decision**: `agentla_board_settings.h`, `agentla_structs.h`, `main.c`, and `tusb_config.h` remain in `firmware/src/`.

**Rationale**: These files are included by code in all three domains. Placing them in a `common/` subdirectory would add a directory level to every `#include` path without real benefit — the root is already the natural "shared" location. The Pico SDK and CMake already resolve includes from `firmware/src/`.

**Alternative considered**: Create `firmware/src/common/`. Rejected because it adds path noise and the root already serves this role.

### 2. SWD goes in `programmer/`, not `common/`

**Decision**: `agentla_swd.c/h` and `agentla_tap_pio.c/h` live in `firmware/src/programmer/`.

**Rationale**: SWD is fundamentally a debug/programming transport. CMSIS-DAP (`agentla_cmsis_dap.c`) depends on SWD, and CMSIS-DAP is a programmer-mode feature. The adapter includes `agentla_swd.h` only for a single `swd_is_initialized()` check — this is a lightweight cross-reference, not a structural dependency.

**Alternative considered**: Place SWD in `common/`. Rejected because it would pull the PIO engine, DMA setup, and ADIv5 protocol code into the "shared" namespace when it's only used by programmer-mode code.

### 3. CMSIS-DAP and USB HID go in `programmer/`

**Decision**: `agentla_cmsis_dap.c/h` and `agentla_usb_hid.c/h` live in `firmware/src/programmer/`.

**Rationale**: CMSIS-DAP is a debug probe protocol — it exposes the SWD interface over USB. USB HID is the transport layer for CMSIS-DAP. Both are programmer-mode features that don't serve analyzer or adapter functionality.

### 4. Include path updates use relative paths from file location

**Decision**: After moving files, `#include` directives reference headers via the project root include path (set in CMakeLists.txt), not relative paths.

**Rationale**: CMakeLists.txt already adds `firmware/src` to the include path. All `#include "agentla_*.h"` directives will continue to work without path prefixes, since the compiler searches the include path. Files within subdirectories that include each other also work because the root is in the include search path.

### 5. CMakeLists.txt uses recursive glob

**Decision**: Update `FILE(GLOB ...)` to use `firmware/src/*.c firmware/src/**/*.c` or explicit subdirectory listings.

**Rationale**: The current glob `firmware/src/*.c` only picks up root-level files. After moving files to subdirectories, the glob must recurse. Using `GLOB_RECURSE` is the standard CMake approach for this.

## Risks / Trade-offs

- **[Include path breakage]** → The build must be tested after every file move. CMake's include path search should handle subdirectory files transparently, but any hardcoded relative paths in `#include` directives will break. Mitigation: grep for all `#include` directives after moving and verify.

- **[Adapter depends on SWD header]** → `agentla_adapter.c` includes `agentla_swd.h` for `swd_is_initialized()`. This creates a cross-module dependency from adapter → programmer. Mitigation: This is acceptable — it's a single function call for a pin-conflict check. If it becomes problematic later, the check can be abstracted into a shared utility.

- **[Empty directories]** → If all files are moved out of root, the root becomes sparse. Mitigation: Root retains 4 essential shared files, so it stays meaningful.
