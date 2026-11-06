# rust-target-flasher Specification

## Purpose
Multi-architecture target identification, control, and in-SRAM FLM flashing runner implemented in Rust on RP2350.

## Requirements

### Requirement: Autonomous Target Detection
The firmware SHALL automatically identify target microcontrollers across ARM Cortex-M (STM32, RP2040/RP2350, Puya PY32) and WCH RISC-V (CH32V003, CH32V203) by querying DBGMCU, IDCODE, or DMI registers.

#### Scenario: STM32F103 detection
- **WHEN** target detection is triggered via CLI `TARGET DETECT` or REST API `GET /api/v1/programmer/target`
- **THEN** the firmware reads DBGMCU IDCODE `0x410` and reports family `STM32F103 Medium Density` with flash size and SRAM boundaries

#### Scenario: CH32V003 detection
- **WHEN** target detection is triggered on a 1-wire target
- **THEN** the firmware initializes SWIO, verifies the QingKe debug module, and reports family `CH32V003`

### Requirement: In-SRAM Algorithm Runner (`algo_runner`)
The firmware SHALL execute CMSIS-Pack FLM algorithm binaries in target SRAM to erase sectors and program pages.

#### Scenario: Sector erase execution
- **WHEN** a sector erase operation is requested on a supported ARM target
- **THEN** the firmware loads the FLM erase blob into target SRAM, sets `R0` to sector address, resumes the core, waits for the `BKPT` halt condition, and verifies a return code of 0

#### Scenario: Page programming execution
- **WHEN** target flash payload data is uploaded
- **THEN** the firmware downloads payload chunks into target SRAM, runs `ProgramPage()`, and verifies written data

### Requirement: Flashing REST API and Telemetry
The firmware SHALL provide REST API endpoints and Server-Sent Events (SSE) for uploading firmware images and monitoring flashing progress.

#### Scenario: Firmware flash via HTTP POST
- **WHEN** a client issues `POST /api/v1/programmer/flash` with a binary firmware payload
- **THEN** the firmware erases target flash, programs the payload, performs a readback verification, and streams progress updates over `/api/v1/events`
