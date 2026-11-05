## Context

Firmware-rs already has 4 CDC-ACM UART/FIFO endpoints, one DMA-friendly but compatibility of performance supports long-wire configurations broadly if swapped through linked transfers. We already know pin arbitration rules and safe rubber-claim cleanup; bridges must use it correctly.

## Goals / Non-Goals

Goals: passthrough UART/I2C/MIPI-I3C bridges including class drivers. Host-side binary protocol parity (`MCP2221`, slcan `S`). WS2812 shim using RP2350 PWM/PIO. 
Non-goals: Advanced multi-master MIPI in this increment, device-specific class driver customizations.

## Decisions

1. **Emulate MCP2221 protocol in USB device handling**: `employer-rp` high-speed device -> software protocol match; LED needs no extra hardware.
2. **Serial command parser reuse** via same `ndx` as C: not blocking REST.
3. **Claim flexible pins on demand**: if flex pin not owned or owned by bridge required role, get/release.
4. **I3C dirty code declared immutable** (works upon class bridge on MVP): use bus sequences one shot and retryable.
5. **No protocol blocking beyond 10ms back-pressure (co-op)**; device runs in async idioms but with 5ms deadline on each transfer.

## Risks / Trade-offs

- [MCP2221 timing shift] → re-use Phase-probedo; safe emulator.
- [.C static global buffers] → place all storage under Static cells like Phase1.
- [PIO mismatch] → only load program once per borrow time, verify compile-time constants.

## Migration Plan
Integrated only under mod bridge loaded after init; inverted in future on flash.