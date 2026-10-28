## Why

While software bitbanging established that the hardware setup and Blue Pill target MCU are 100% working, pure PIO SWD operation (`swd_read_idcode()`) returns `raw_ack = 0xE0000000` because the PIO DMA buffer layout, autopush/autopull bit shifting, and `adiv5_check_ack` comparison logic require precise hardware alignment. We need to debug and perfect the PIO engine so that pure PIO SWD achieves `ACK = 001 (OK)` and `DPIDR = 0x1BA01477`.

## What Changes

- Debug and refactor `firmware/pio/swd.pio` and `firmware/src/programmer/swd.c` PIO buffer alignment.
- Correct `swd_transfer` TX FIFO word layout for request, ACK checking, and data transfer phases.
- Ensure pure PIO SWD (`SWD PIO_IDCODE` / `swd_read_idcode()`) succeeds with `ACK = 1` and `DPIDR = 0x1BA01477` without relying on software bitbanging fallbacks.

## Capabilities

### New Capabilities
- `stm32f1-pio-swd-debug`: Requirements for pure PIO-based SWD protocol execution on RP2350 PIO0, hardware ACK checking, and full target detection.

### Modified Capabilities
- None.

## Impact

- `firmware/pio/swd.pio`: Refactored PIO sub-programs (`swd_seq_out`, `swd_seq_in`, `adiv5_check_ack`).
- `firmware/src/programmer/swd.c`: Fixed `swd_transfer` FIFO buffer construction and RX reading logic.
- `firmware/src/programmer/tap_pio.c`: Configured DMA and SM shift register parameters.
