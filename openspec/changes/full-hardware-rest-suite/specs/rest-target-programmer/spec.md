## ADDED Requirements

### Requirement: Autonomous Target MCU Identification
The HTTP server SHALL provide `GET /api/v1/programmer/target` to query connected debug targets over SWD, RVSWD, or SWIO and return identified MCU architecture, family, flash capacity, SRAM capacity, and protection level.

#### Scenario: Detecting connected STM32 target
- **WHEN** client sends `GET /api/v1/programmer/target` with an STM32F103 target connected
- **THEN** server returns HTTP 200 with `{ "detected": true, "arch": "cortex-m3", "family": "STM32F1", "part": "STM32F103C8", "flash_kb": 64, "sram_kb": 20, "rdp_level": 0 }`.

### Requirement: Target Reset and Boot Control
The HTTP server SHALL provide `POST /api/v1/programmer/reset` accepting reset modes (`hard`, `soft`, `bootloader`) driving target hardware `nRST` and `BOOT0` lines.

#### Scenario: Pulsing target reset line
- **WHEN** client sends `POST /api/v1/programmer/reset` with `{ "mode": "hard", "duration_ms": 50 }`
- **THEN** server asserts target nRST low for 50ms, releases to float, and returns HTTP 200 with `{ "status": "reset_complete" }`.

### Requirement: Flash Memory Erase and Program
The HTTP server SHALL provide `POST /api/v1/programmer/erase` and `POST /api/v1/programmer/flash` to execute sector or mass erases and write target flash memory via on-chip RAM algorithm runners.

#### Scenario: Performing target mass erase
- **WHEN** client sends `POST /api/v1/programmer/erase` with `{ "mode": "mass" }`
- **THEN** server unlocks flash controller, triggers mass erase, verifies blank state, and returns HTTP 200 with `{ "status": "erased", "mode": "mass" }`.
