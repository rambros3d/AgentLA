## Context

AgentLA dynamically reconfigures flexible GPIO pins at runtime via REST calls (e.g. assigning pins to I2C, SPI, UART, PWM, or Logic Analyzer). The `pin_owner` module enforces that no two drivers claim the same pin simultaneously and that requested pins have the required hardware capability.

This design extracts the tracking table, capability checking, and claim transactions into `crates/agentla-pin-arbiter`.

## Goals / Non-Goals

**Goals:**
- Extract `Driver` enum, capability flags (`CAP_*`), `ClaimError`, and `SlotId`.
- Extract `PinRegistry` managing up to 32 pins and 16 active slots using fixed-capacity `heapless` arrays without heap allocation.
- Introduce `PinQuiesce` trait for notifying the hardware layer when a pin is released.
- 100% test coverage on `x86_64` for overlapping claims, invalid capabilities, and partial rollback.

**Non-Goals:**
- Removing the RP2350 SIO pad register manipulation from `firmware-rs`.
