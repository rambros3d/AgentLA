## Why

Different host debug tools express clock rates differently:
- ST-Link tools send fixed frequency divisor table indices (`0 = 4 MHz`, `1 = 1.8 MHz`, `2 = 1.2 MHz`, etc.).
- CMSIS-DAP tools send target clock rates in Hertz via `DAP_SWJ_Clock`.
- WCH-Link tools send speed tier codes (`1 = 6 MHz`, `2 = 4 MHz`, `3 = 400 kHz`).

Currently, `agentla-programmer` uses rough approximations with integer-only PIO clock dividers, and WCH RVSWD has a fixed hardcoded divider (`36`). Adapting the clock divisor mapping and frequency calibration architecture from `ref/hybridlink` ([`app/stlink.c`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/app/stlink.c) and [`port/stm32f1/hal.c`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/port/stm32f1/hal.c)) enables exact fractional PIO clock synthesis across all three debug personalities, preventing communications timeouts on slow targets and signal distortion at high speeds.

## What Changes

- **Calibrated Clock Synthesis**: Implement `calculate_pio_clkdiv(sys_clk_hz, target_freq_hz, cycles_per_bit)` computing exact 16-bit integer and 8-bit fractional dividers for RP2350 PIO state machines.
- **ST-Link Divisor Mapping**: Implement bidirectional conversion between ST-Link divisor indices (0..798) and safe target frequencies (4.0 MHz down to 5 kHz) adhering to the rule that the synthesized clock never exceeds the requested rate.
- **WCH-Link Speed Tier Execution**: Wire WCH command `0x0C` to dynamically reconfigure the RVSWD PIO clock divider for 6 MHz (speed 1), 4 MHz (speed 2), and 400 kHz (speed 3).
- **Dynamic PIO Clock Updating**: Implement `set_clock` on `RvswdTransport` and update `SwdTransport` to program both integer and fractional dividers into `pac::PIO0.sm(x).clkdiv()`.

## Capabilities

### New Capabilities
- `clock-frequency-calibration`: High-precision clock mapping and fractional PIO divider synthesis across ST-Link divisor tables, CMSIS-DAP Hz requests, and WCH speed tiers.

### Modified Capabilities
- `stlink-probe-endpoint`: Update `DBG_APIV2_SWD_SET_FREQ` to use calibrated divisor-to-frequency mapping.
- `wchlink-probe-endpoint`: Update command `0x0C` to dynamically update RVSWD PIO clock frequency.

## Impact

- **Crates**:
  - `crates/agentla-programmer`: Add `clock.rs` with `calculate_pio_clkdiv`, `swd_divisor_to_khz`, `wch_speed_to_khz`; update `pio/swd.rs` and `pio/rvswd.rs` to write fractional clkdiv; update `stlink.rs` and `wch.rs`.
- **Clock Precision**: Eliminates clock drift and timing jitter on SWCLK and RVSWD CLK pins across all target microcontrollers.
