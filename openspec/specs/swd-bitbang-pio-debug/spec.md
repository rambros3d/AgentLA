# swd-bitbang-pio-debug Specification

## Purpose
TBD - created by archiving change swd-bitbang-pio-debug. Update Purpose after archive.
## Requirements
### Requirement: Software Bitbanging SWD Diagnostic Command
The system SHALL provide a software bitbanging SWD driver and CDC serial command `SWD BITBANG` that directly controls GPIO4 (SWCLK) and GPIO5 (SWDIO) to read `DPIDR` without using the PIO state machine.

#### Scenario: Execute SWD BITBANG command
- **WHEN** the user sends `SWD BITBANG\r\n` over CDC serial
- **THEN** the probe executes pure C bitbanging and returns `OK SWD BITBANG ack=<val> idcode=0x<IDCODE>` or an error response detailing the exact captured ACK bits.

### Requirement: PIO SWD State Machine Verification
The system SHALL use the empirical findings from Bitbanging to update `firmware/pio/swd.pio` and `firmware/src/programmer/swd.c` so that the PIO state machine achieves `ACK = 001 (OK)` and `DPIDR = 0x1BA01477`.

#### Scenario: Verify PIO SWD IDCODE after fix
- **WHEN** the user sends `SWD IDCODE\r\n` over CDC serial after PIO refactoring
- **THEN** the probe returns `OK SWD IDCODE 0x1BA01477`.

