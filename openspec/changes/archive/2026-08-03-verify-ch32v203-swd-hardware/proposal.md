## Why

To guarantee robust, reliable hardware debugging and flash programming for WCH CH32V203 microcontrollers, we need an automated end-to-end hardware verification suite that tests 2-wire RV-SWD link synchronization, DTM/DMI register operations, flash page erasures, multi-byte binary programming, and readback verification on physical target hardware.

## What Changes

- Add automated Python hardware verification script `test/test_ch32v203_hardware_verification.py` targeting `/dev/ttyACM0`.
- Add test routines for RV-SWD DP/AP transport handshake, RISC-V Debug Module status (`DMSTATUS` / `DMCONTROL`), CPU halt and resume, memory read/write, fast-page flash unlock (`FLASH_MODEKEYR`), page erase, and exact binary payload verification.
- Add hardware diagnostics and timing check verification.

## Capabilities

### New Capabilities
- `ch32v203-swd-verification`: Automated hardware test suite and verification protocol for CH32V203 2-wire RV-SWD debugging and flash programming.

### Modified Capabilities

## Impact

- Test suite: `test/test_ch32v203_hardware_verification.py`.
- Documentation: Verification procedures for CH32 RISC-V targets.
