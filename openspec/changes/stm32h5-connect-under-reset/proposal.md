## Why

On ARMv8-M Cortex-M33 targets such as the STM32H503, user firmware or empty/corrupted flash can disable SWD pins (`PA13`/`PA14`), enter low-power sleep (`STOP`/`STANDBY`), or enter core lockup within microseconds of reset release. Currently, `flash_target_port_global` and `erase_flash_port_global` attempt to attach and halt the core while running without hardware reset control, leading to immediate SWD disconnect (`ACK = 15`) or reading dummy zeros (`0x00000000`) from an unclocked AHB bus when held in static reset. Hardened cycle-0 vector catch under reset is required to halt the target before user code executes.

## What Changes

- Modify `CortexM::connect_under_reset` to support ARMv8-M / STM32H5 dual-AP architecture:
  - Route low-power debug clock configuration (`DBGMCU_CR |= 0x06`) through AP 0 (`0xE00E4004`) while under physical reset.
  - Arm vector catch (`DEMCR.VC_CORERESET` and `DHCSR.C_DEBUGEN | C_HALT`) across reset release.
  - Re-establish SWD sync on AP 1 (`AHB-AP`) with Non-Secure privileged CSW (`0x23000012`).
- Update `flash_target_port_global` and `erase_flash_port_global` in `firmware-rs/src/programmer/mod.rs` to automatically use `connect_under_reset` when attaching to STM32H5 targets or when standard attach fails.
- Ensure post-programming verification readback and GDB memory access operate with the target properly halted at instruction 0.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `cortex-m-vector-catch`: Extend connect-under-reset and vector catch requirements to support ARMv8-M dual-AP targets (STM32H5) with AP 0 DBGMCU configuration and AP 1 AHB-AP cycle-0 halting.

## Impact

- `crates/agentla-programmer/src/target/cortex_m.rs`: `connect_under_reset` implementation for ARMv8-M.
- `firmware-rs/src/programmer/mod.rs`: `flash_target_port_global`, `erase_flash_port_global`, and `connect_under_reset_port_global`.
- End-to-end flashing and SWD verification of STM32H503 on N20 Servo board.
