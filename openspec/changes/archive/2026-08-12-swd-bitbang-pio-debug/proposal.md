## Why

To definitively isolate whether the SWD target communication failure is caused by physical line timing / hardware setup versus PIO state machine configuration, we need a pure software Bitbanging SWD driver and diagnostic command (`SWD BITBANG`). Once Bitbanging proves working target ACK latching, we will use that exact timing to fix the PIO state machine.

## What Changes

- Add pure C software bitbanging SWD driver (`swd_bitbang_read_idcode()`) in `firmware/src/programmer/swd.c`.
- Expose CDC serial CLI command `SWD BITBANG` in `firmware/src/main.c`.
- Use Bitbanging diagnostic results to identify the exact PIO timing gap and fix `firmware/pio/swd.pio` and `firmware/src/programmer/swd.c`.

## Capabilities

### New Capabilities
- `swd-bitbang-pio-debug`: Defines requirements for software bitbanging SWD diagnostic commands, isolation of PIO timing/pin direction issues, and PIO state machine refactoring for Blue Pill target verification.

### Modified Capabilities
- None.

## Impact

- `firmware/src/programmer/swd.c`: Added `swd_bitbang_read_idcode()` implementation.
- `firmware/src/main.c`: Added `SWD BITBANG` command handler.
- `firmware/pio/swd.pio`: Refactored PIO state machine instructions based on bitbanging timing findings.
