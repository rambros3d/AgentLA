## Context

`firmware-rs` provides three high-performance host bridging protocols that allow AgentLA to act as standard Linux peripherals:
1. `can_gsusb.rs`: Conforms to Linux `drivers/net/can/usb/gs_usb.c`.
2. `i2c_tinyusb.rs`: Conforms to Linux `drivers/i2c/busses/i2c-tiny-usb.c`.
3. `slcan.rs`: Conforms to the Lawicel SLCAN ASCII standard used by `slcand` and `can-utils`.

Currently, these files mix protocol packet layout, control request dispatching, bit timing calculation, and board pin configuration. This design extracts the pure protocols into `crates/agentla-bridges`.

```
┌────────────────────────────────────────────────────────┐
│                      firmware-rs                       │
│    (USB descriptors, EP0 control transfer callbacks,   │
│     pin_owner claims, board-specific transceiver GPIO) │
└───────────────────────────┬────────────────────────────┘
                            │ depends on
                            ▼
┌────────────────────────────────────────────────────────┐
│                crates/agentla-bridges                  │
│                                                        │
│ • i2c_tinyusb: Control request dispatcher, status      │
│   flags, SMBus emulation, over embedded-hal I2C        │
│ • gs_usb: Host format structs, bittiming math,         │
│   mode transitions, embedded-can frame packing         │
│ • slcan: S0..S8 bittiming table, ASCII parser/emitter, │
│   standard/extended frame conversion                   │
│                                                        │
│ (Pure #![no_std], optional defmt & std for tests)      │
└────────────────────────────────────────────────────────┘
                            ▲
                            │
              ┌─────────────┴─────────────┐
              │  Host Unit Tests (x86_64) │
              │  Tests with mock I2C/CAN  │
              └───────────────────────────┘
```

## Goals / Non-Goals

**Goals:**
- Extract `gs_usb`, `i2c_tinyusb`, and `slcan` protocol engines into `crates/agentla-bridges`.
- Follow Embassy / Rust Embedded standards:
  - `i2c-tiny-usb` operates over `embedded-hal` / `embedded-hal-async` I2C traits.
  - `gs_usb` models standard CAN frames conforming to `embedded-can`.
  - `slcan` parses byte buffers / streams with zero dynamic allocation.
  - Optional `defmt` feature for Embassy logging.
- Enable full host unit test coverage for protocol packets, bit timing formulas, and command dispatch.
- Integrate back into `firmware-rs` with zero regression to USB functionality or memory footprint.

**Non-Goals:**
- Moving USB hardware endpoint drivers or USB descriptor trees out of `firmware-rs` (those belong to `embassy-usb` in the top-level binary).
- Changing pin allocation or transceiver management in `pin_owner`.

## Decisions

### Decision 1: Protocol Engine vs Transceiver Driver
- **Rationale**: The crate handles packet definitions, control request handling, and protocol translation. The physical transceiver enable/standby/termination pins remain orchestrated by `firmware-rs` via `pin_owner` and `agentla-bsp`.

### Decision 2: Trait-First I/O for I2C Tiny USB
- **Rationale**: The `I2cTinyUsbHandler` provides a dispatch method:
  ```rust
  pub fn handle_control_request<I2C: embedded_hal::i2c::I2c>(
      &mut self,
      i2c: &mut I2C,
      request: u8,
      value: u16,
      index: u16,
      data: &mut [u8],
  ) -> Result<usize, I2cTinyUsbError<I2C::Error>>
  ```
  This allows host unit tests with a `MockI2c` and embedded execution with Embassy's I2C driver.

### Decision 3: SLCAN Parser Architecture
- **Rationale**: Implement SLCAN frame parsing from ASCII slices into strongly typed `SlcanCommand` (Open, Close, SetupSpeed, SendStandard, SendExtended).

## Risks / Trade-offs

- **[Risk] Alignment and struct packing for Linux gs_usb structs**
  → *Mitigation*: Use `#[repr(C, packed)]` matching `gs_usb.h` from the Linux kernel and add unit tests verifying exact `core::mem::size_of` for each struct.
- **[Risk] Embassy async vs sync I2C in EP0 callback**
  → *Mitigation*: Support sync `embedded-hal::i2c::I2c` for immediate EP0 execution as currently used in `firmware-rs`, with extension points for async.
