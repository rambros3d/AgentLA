## ADDED Requirements

### Requirement: Firmware source files SHALL be organized by functional domain
All `.c` and `.h` files in `firmware/src/` SHALL be placed in the subdirectory matching their functional domain. Shared infrastructure files (board settings, structs, main, USB config) SHALL remain in the `firmware/src/` root.

#### Scenario: Analyzer files in analyzer directory
- **WHEN** a file implements signal capture or protocol decoding
- **THEN** it SHALL be located in `firmware/src/analyzer/`

#### Scenario: Adapter files in adapter directory
- **WHEN** a file implements I2C or UART bus adapter functionality
- **THEN** it SHALL be located in `firmware/src/adapter/`

#### Scenario: Programmer files in programmer directory
- **WHEN** a file implements SWD, CMSIS-DAP, target detection, flash programming, or RISC-V debug
- **THEN** it SHALL be located in `firmware/src/programmer/`

#### Scenario: Shared files in root
- **WHEN** a file defines board settings, shared types, or is the main command router
- **THEN** it SHALL be located in `firmware/src/`

### Requirement: Build system SHALL compile all source files across subdirectories
The CMakeLists.txt `FILE(GLOB ...)` or equivalent SHALL recursively discover all `.c` files under `firmware/src/` including subdirectories.

#### Scenario: Recursive source discovery
- **WHEN** the build system runs
- **THEN** it SHALL compile all `.c` files in `firmware/src/`, `firmware/src/analyzer/`, `firmware/src/adapter/`, and `firmware/src/programmer/`

### Requirement: Include paths SHALL resolve across subdirectories
All `#include "agentla_*.h"` directives SHALL resolve correctly after files are moved. The CMake include path (`firmware/src`) SHALL be sufficient for all includes.

#### Scenario: Cross-directory includes
- **WHEN** a file in `firmware/src/programmer/` includes `agentla_board_settings.h`
- **THEN** the include SHALL resolve via the `firmware/src` include path without requiring relative path prefixes
