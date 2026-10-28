## Why

While bitbanging established 100% working SWD communication with the Blue Pill STM32F103 target (`IDCODE = 0x1BA01477`), pure PIO SWD (`swd_read_dp(DP_DPIDR)`) fails because of SWCLK frequency mismatch (1 MHz vs 100 kHz bitbanging) and turnaround bit phase alignment. We need to achieve pure PIO SWD execution (`SWD PIO_IDCODE`) with zero bitbanging fallbacks.

## What Changes

- Set PIO state machine frequency to 100 kHz for SWJ-DP reset and turnaround sequence.
- Refactor `swd.pio` sub-program turnaround clock transitions and hardware ACK matching.
- Verify SWD requests and responses using built-in logic analyzer capture (`SWD CAPTURE`) on GPIO20 (SWCLK) and GPIO21 (SWDIO).
- Ensure `SWD PIO_IDCODE` returns `OK SWD PIO_IDCODE 0x1BA01477` directly over PIO0.

## Capabilities

### New Capabilities
- `stm32f1-pure-pio-swd`: Pure PIO0 hardware driver for ARM ADIv5 SWD protocol execution on STM32F103 Blue Pill target.

### Modified Capabilities
- None.

## Impact

- `firmware/pio/swd.pio`: Refactored PIO sub-programs (`swd_seq_out`, `swd_seq_in`, `swd_seq_in_turnaround`).
- `firmware/src/programmer/swd.c`: Configured 100 kHz default SWD clock frequency and DMA FIFO layout.
