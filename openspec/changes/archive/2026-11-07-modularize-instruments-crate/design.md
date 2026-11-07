## Context

The instrument subsystem in `firmware-rs` provides:
1. An analog oscilloscope reading RP2350 SAR ADC circular buffers and computing metrics.
2. A signal generator programming RP2350 PWM slices to output clocks/pulses.
3. A multi-channel servo engine calculating microsecond duty cycles and divisors.

Currently, the pure algorithms (integer math, fixed-point dividers, RMS/Vpp metrics) are interspersed with `pac::PWM`, `pac::ADC`, and `pin_owner` register accesses. This design decouples the algorithms into `crates/agentla-instruments`.

## Goals / Non-Goals

**Goals:**
- Extract pure math and DSP functions to `crates/agentla-instruments`:
  - `OscilloscopeMetrics`: `compute_metrics(samples: &[u16], vref_mv: u32, sample_rate_hz: u32) -> Metrics`
  - `TriggerDetector`: threshold and edge trigger evaluation on `&[u16]`.
  - `PwmTimingMath`: pure calculation of `(div_int, div_frac, top)` from `(sys_clk, freq)`.
  - `ServoMath`: pure calculation of pulse width to compare counts for 50 Hz and variable frequency modes.
- Achieve 100% host unit test coverage on `x86_64` without hardware.
- Zero heap allocation (`no_std` first).

**Non-Goals:**
- Moving hardware DMA interrupt handlers or hardware PAC register writes out of `firmware-rs`.

## Decisions

### Decision 1: Pure Functions Operating on Slices
- All DSP functions accept borrowed sample slices (`&[u16]`). No buffers are allocated inside the crate.

### Decision 2: Math Module Segregation
- Group into clean submodules: `scope`, `generator`, `servo`.
