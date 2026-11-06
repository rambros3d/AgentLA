## 1. Phase 1: Connect-under-Reset & Vector Catch Engine

- [x] 1.1 Implement register constants (`DEMCR_TRCENA`, `DEMCR_VC_CORERESET`, `AIRCR_VECTKEY`, `AIRCR_SYSRESETREQ`) and `connect_under_reset` in `firmware-rs/src/programmer/target/cortex_m.rs`
- [x] 1.2 Implement `reset_and_catch` software trigger in `cortex_m.rs` and wire into `ProgrammerPins` and `programmer/mod.rs`
- [x] 1.3 Add REST API endpoint `POST /api/v1/programmer/reset-catch` in `firmware-rs/src/rest.rs`
- [x] 1.4 Add unit tests in `test/test_cortex_reset_catch.py` and verify on live RP2350 probe

## 2. Phase 2: Vendor UART Bootloader Recovery Engine (STM32 AN3155)

- [x] 2.1 Implement `Stm32Bootloader` module in `firmware-rs/src/programmer/bootloader/stm32.rs` with `8E1` UART configuration and `0x7F` auto-baud sync
- [x] 2.2 Implement AN3155 command handlers: Readout Unprotect (`0x92`), Get Commands (`0x00`), Extended Erase (`0x44`), Write Memory (`0x31`)
- [x] 2.3 Wire hardware orchestration sequence (`PIN_BOOT0` assertion, `PIN_RESET` pulse, UART parity setup and teardown) in `pins.rs` and `programmer/mod.rs`
- [x] 2.4 Add REST API endpoint `POST /api/v1/programmer/rescue/stm32` in `firmware-rs/src/rest.rs` with `PinArbiter` conflict protection
- [x] 2.5 Add unit test suite in `test/test_stm32_bootloader.py` validating frame formatting, checksums, and timeout handling

## 3. Phase 3: Dynamic CMSIS-Pack FLM Runner on LittleFS

- [x] 3.1 Create host packager utility `tools/pack_flm.py` to parse `.FLM` and ELF binaries into `AlgoBlobHeader` binary format
- [x] 3.2 Update `firmware-rs/src/programmer/algo/mod.rs` to read `.bin` blobs from LittleFS Partition 1 (`/system/algo/<name>.bin`) via `SYSTEM_FS`
- [x] 3.3 Implement SRAM execution harness in `FlashAlgorithmRunner` verifying `BKPT 0` halt, R0 return code, and stack top pointer
- [x] 3.4 Package initial STM32F1/G0 algorithm blob into `SYSTEM_FS` and verify end-to-end flash erase and write

## 4. UI Integration & Documentation

- [x] 4.1 Add "Connect Under Reset" toggle and "STM32 Unbrick / Recovery" trigger button to Programmer Web Desktop tool in `web-ui/`
- [x] 4.2 Update `FUTURE.md` and documentation to record completed milestones
