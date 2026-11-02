# dynamic-board-manifest Specification

## Purpose
TBD - created by archiving change dynamic-firmware-webui. Update Purpose after archive.
## Requirements
### Requirement: Declarative Board Manifest Definition
The system SHALL provide a structured JSON specification (`board.json`) for each supported hardware profile defining MCU attributes, peripheral capabilities, electrical rails, level translation, and physical zone pin allocations.

#### Scenario: Valid board manifest schema
- **WHEN** `board.json` is parsed by build tools or the WebUI client
- **THEN** it SHALL contain `board_id`, `name`, `version`, `assets`, `mcu`, `features`, and `zones` fields conforming to the AgentLA board schema

### Requirement: Pre-build C Header and Asset Generation
CMake SHALL automatically execute a code generation pipeline before firmware compilation to generate low-level C headers and stage web assets based on the selected board profile.

#### Scenario: Pre-build execution on board selection
- **WHEN** CMake configures or builds with a target board (e.g. `BAKUPROBE_BOARD=ON` or `BOARD_TYPE=BOARD_PICO_2`)
- **THEN** the generator script SHALL output `generated/board_config.h` containing pin constants and feature macros, and copy `board.json`, `logo.svg`, and `pinout.svg` into the lwIP fsdata staging directory

### Requirement: Zero-RAM Static Manifest Serving via lwIP
The firmware HTTP server SHALL embed and serve `/board.json` directly from flash ROM using lwIP fsdata without allocating runtime dynamic RAM or invoking C request handlers.

#### Scenario: Client requests board manifest
- **WHEN** the WebUI client performs an HTTP `GET /board.json` request
- **THEN** the firmware HTTP server SHALL respond with HTTP 200, Content-Type `application/json`, and the exact minified JSON payload stored in flash

### Requirement: Client Cache-Busting Verification
The WebUI client SHALL fetch `/board.json` with a cache-busting version parameter to prevent stale board definitions from being reused when hot-swapping between different probe hardware.

#### Scenario: Connecting to probe
- **WHEN** the WebUI initializes or reconnects to the probe
- **THEN** it SHALL append a query parameter or timestamp to `/board.json` and verify that the received `board_id` matches the active runtime configuration

