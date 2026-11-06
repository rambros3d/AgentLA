## Why

Embedded developers and automated test benches frequently encounter target lockups when targets execute buggy firmware, enter immediate sleep modes (`WFI`), disable debug pins, or get locked with Readout Protection (RDP Level 1). Currently, AgentLA's SWD engine attaches after reset release, leaving a race window where crashed firmware can wedge the bus before the probe can halt the core (`PC = 0xFFFFFFFE` fault). Furthermore, recovering locked targets requires vendor-specific bootloader orchestration over UART, and expanding chip support requires hardcoding flash peripheral drivers into probe ROM. 

This change delivers a complete 3-phase target debugging and flashing expansion:
1. **Phase 1**: Hardened Connect-under-Reset and Vector Catch (`DEMCR.VC_CORERESET`) to halt Cortex-M targets on cycle 0 before any code executes.
2. **Phase 2**: Vendor UART Bootloader Recovery Engine for STM32 targets (AN3155 protocol via `BOOT0` + `nRST` + target UART) to perform automated unprotect and mass-erase unbricking.
3. **Phase 3**: Dynamic CMSIS-Pack FLM Algorithm Runner loading pre-packaged binary blobs from LittleFS Partition 1 (`/system/algo/`), enabling universal flash support across thousands of ARM MCUs without firmware ROM bloat.

## What Changes

- **Cortex-M Connect-under-Reset Engine**: Implements physical `nRST` assertion during SWD init, arming `DEMCR.VC_CORERESET` (Core Reset Vector Catch) and `DEMCR.TRCENA` before releasing reset, guaranteeing target halt on the reset vector.
- **Software Reset Execution**: Adds support for software-triggered resets via `AIRCR.SYSRESETREQ` and `AIRCR.VECTRESET` over SWD.
- **STM32 AN3155 UART Bootloader Recovery**: Adds dedicated hardware sequencing of `PIN_BOOT0` (GPIO 2) and `PIN_RESET` (GPIO 6/10/13) combined with target UART0 (GPIO 8 TX / 14 RX) in `8E1` parity mode:
  - Auto-baud synchronization (`0x7F`).
  - Readout Unprotect command (`0x92` / `0x6D`) triggering automatic silicon mass-erase.
  - Sector Erase (`0x43` / `0xBC`) and Extended Erase (`0x44` / `0xBB`).
  - Memory Write (`0x31` / `0xCE`) and Read (`0x11` / `0xEE`).
- **REST & MCP Recovery API**: Exposes `POST /api/v1/programmer/rescue/stm32` and `POST /api/v1/programmer/reset-catch` endpoints for WebUI and autonomous agent unbricking.
- **Dynamic LittleFS Flash Algorithm Loader**: Extends `FlashAlgorithmRunner` to read algorithm blobs directly from LittleFS Partition 1 (`/system/algo/<chip>.bin`), allocating target RAM buffers and executing `Init`, `EraseSector`, `ProgramPage`, and `UnInit` routines.
- **Host Packager Utility**: Adds `tools/pack_flm.py` to convert Keil/CMSIS-Pack `.FLM` and position-independent `.ELF` executables into compact `AlgoBlobHeader` binary blobs ready for deployment to LittleFS.

## Capabilities

### New Capabilities
- `cortex-m-vector-catch`: Hardware connect-under-reset, `DEMCR.VC_CORERESET` vector catch configuration, and `AIRCR` system reset execution.

### Modified Capabilities
- `bootloader-recovery`: Adds ST AN3155 protocol engine (`8E1`, `0x7F` auto-baud, `0x92` RDP unprotect, mass erase), dedicated hardware pin orchestration (`BOOT0` + `nRST`), and REST endpoints.
- `generic-cortexm-algo-runner`: Adds LittleFS `/system/algo/` storage resolution, dynamic binary `AlgoBlobHeader` parsing, and RAM execution lifecycle.

## Impact

- **Firmware (`firmware-rs`)**:
  - `src/programmer/target/cortex_m.rs`: Adds vector catch, reset control, and register definitions.
  * `src/programmer/pins.rs`: Adds orchestrated bootloader reset sequences.
  * `src/programmer/bootloader/mod.rs` & `stm32.rs`: New AN3155 bootloader client.
  * `src/programmer/algo/mod.rs`: File-based algorithm blob loader via `SYSTEM_FS`.
  * `src/rest.rs`: New rescue and reset-catch REST API routes.
- **Host Tools (`tools/`)**:
  - `tools/pack_flm.py`: Offline ELF/FLM to `AlgoBlobHeader` packager.
- **Storage**:
  - Partition 1 (`SYSTEM_FS`) hosts `/system/algo/*.bin` blobs.
- **Web UI (`web-ui/`)**:
  - Programmer workbench adds "Connect Under Reset", "Vector Catch", and "STM32 Unbrick / Recovery" action controls.
