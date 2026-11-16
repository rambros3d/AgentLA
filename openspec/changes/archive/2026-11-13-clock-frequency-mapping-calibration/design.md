## Context

In hardware debugging, communication reliability depends on providing exact, clean clock signals. If a probe clocks faster than requested, target shift registers can miss setup/hold windows. If the clock is too slow, firmware flash operations take excessive time.

In `ref/hybridlink` ([`app/stlink.c`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/app/stlink.c) and [`port/stm32f1/hal.c`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/port/stm32f1/hal.c)):
- ST-Link command `DBG_APIV2_SWD_SET_FREQ` maps host divisor values: `div == 0` yields 4.0 MHz, and `div > 0` yields `3600 kHz / (div + 1)`.
- WCH-Link speed settings (`0x0C`) specify 3 tiers: 6.0 MHz (high), 4.0 MHz (medium), and 400 kHz (low).
- CMSIS-DAP commands (`DAP_SWJ_Clock`) specify clock rate directly in Hz.

RP2350 PIO state machines feature high-resolution 16-bit integer and 8-bit fractional clock dividers (`clkdiv`). By calculating both integer and fractional components, AgentLA can synthesize clock frequencies with sub-percent accuracy without clock jitter.

## Goals / Non-Goals

**Goals:**
- Provide exact fractional divider calculation `calculate_pio_clkdiv(sys_clk_hz, target_freq_hz, cycles_per_bit) -> (u16, u8)`.
- Implement `swd_divisor_to_khz` and `wch_speed_to_khz` mapping utilities.
- Update `SwdTransport` to program both integer and fractional dividers into `pac::PIO0.sm(0).clkdiv()`.
- Add `set_clock` to `RvswdTransport` and wire WCH command `0x0C` to dynamically reconfigure the RVSWD PIO clock divider.
- Ensure synthesized clocks are strictly less than or equal to the requested rate.

**Non-Goals:**
- Exceeding the maximum target interface limits (4 MHz for Cortex-M0/M33 baseline, 6 MHz for CH32V).

## Decisions

### Decision 1: High-Resolution Fractional PIO Divider Calculation

- **Choice**:
  ```rust
  let total_substeps = (sys_clk_hz as u64 * 256) / (target_freq_hz as u64 * cycles_per_bit as u64);
  let int_div = ((total_substeps >> 8) as u16).max(1);
  let frac_div = (total_substeps & 0xFF) as u8;
  ```
- **Rationale**:
  - Utilizing the 8-bit fractional divider eliminates rounding errors that occur with integer-only division.
  - Guarantees the resulting clock is accurate and never overclocks the target.

### Decision 2: Centralized Clock Calibration Module

- **Choice**: Place all frequency mapping functions in `crates/agentla-programmer/src/clock.rs`.
- **Rationale**: Keeps math and protocol mapping tables pure, testable on host architectures without PIO hardware dependencies.

## Risks / Trade-offs

- **[Risk] High-fractional jitter on low division factors**
  - *Mitigation*: For high frequencies (e.g. 4-6 MHz from a 150 MHz system clock), division ratios are relatively large ($150 / 8 \approx 18.75$), where fractional duty cycle variation is negligible.
