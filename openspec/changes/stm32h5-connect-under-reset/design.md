## Context

On the custom N20 Servo board, the STM32H503KBU6 target microcontroller executes firmware that disables SWD pins or enters deep sleep within microseconds of reset release. The target cannot be attached to or halted while running normally (`ACK = 15`). While held in reset, the AHB bus clock is gated and returns all zeros. To reliably program flash and attach GDB, the programmer must halt the core at cycle 0 of reset recovery.

## Goals / Non-Goals

**Goals:**
- Implement cycle-0 hardware vector catch and connect-under-reset for ARMv8-M / STM32H5 dual-AP targets.
- Access DBGMCU via AP 0 (`0xE00E4004`) while under physical reset to un-gate low-power debug clocking (`DBG_STOP | DBG_STANDBY`).
- Arm vector catch and burst-halt across reset release, catching the target before user firmware runs.
- Use connect-under-reset in `flash_target_port_global` and `erase_flash_port_global` so flash programming succeeds end-to-end.
- Preserve `#![no_std]` and zero-heap across all crates with zero emojis.

**Non-Goals:**
- Supporting hot-plug attach without nRST wire for targets that aggressively disable SWD pins (nRST wire is required for cycle-0 catch).
- Modifying target application firmware binaries.

## Decisions

### Decision 1: Route under-reset DBGMCU writes through AP 0
On STM32H5, while physical `nRST` is held low, the system AHB bus matrix is in reset and accesses to `0x44024004` via AP 1 fail. However, AP 0 (APB-AP) connects directly to CoreSight debug APB, where DBGMCU is mapped at `0xE00E4004`.
*Rationale*: Writing `0x06` to `0xE00E4004` via AP 0 ensures debug clocks remain running through reset release.

### Decision 2: Burst vector catch and re-sync across reset release
Upon releasing `nRST` high:
1. Re-run `read_dpidr_retry` and `power_up_debug` to re-sync DP.
2. Select AP 1 (`AHB-AP`) with `CSW = 0x23000012`.
3. Halt the core at instruction 0 (`DHCSR` and `DEMCR`).
*Alternative considered*: Polling for halt without re-syncing DP. Rejected because reset bounce can drop line sync.

### Decision 3: Integrate connect-under-reset into flash workflow
In `firmware-rs/src/programmer/mod.rs`, when `is_h5` is detected or when normal attach fails, use `connect_under_reset` so the target core is cleanly halted before calling `FlashAlgorithmRunner::stm32h5_erase_sector` and `stm32h5_program_page`.

## Risks / Trade-offs

- [Target nRST pin disconnected] -> Verify pin state with `swj_pins`; if nRST cannot be asserted, fall back to soft reset catch.
- [SWD line capacitance causing sync loss across edge] -> Allow 5ms settling delay after releasing nRST before initiating SWD read.
