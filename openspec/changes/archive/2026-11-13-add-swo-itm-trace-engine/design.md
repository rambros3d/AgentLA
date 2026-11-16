## Context

Serial Wire Output (SWO) is a single-pin asynchronous serial line (NRZ/UART) output by ARM Cortex-M microcontrollers (such as STM32, RP2350, SAMD, NRF52) from the CoreSight Instrumentation Trace Macrocell (ITM) and Data Watchpoint and Trace (DWT) units. Typical use cases include high-throughput `printf` logging without CPU cycle penalties and statistical program counter (PC) sampling.

In genuine ST-Link/V2-1 and `ref/hybridlink`, SWO is streamed over USB Endpoint 0x82. To prevent host tool errors (such as STM32CubeIDE's *"SWV info, Failed to read data!!!"*), the firmware must strictly coordinate between promised byte counts (`GET_TRACE_NB`) and actual packets transmitted on EP 0x82.

## Goals / Non-Goals

**Goals:**
- Provide a robust SWO trace capture engine supporting baud rates up to 4.5 Mbaud (standard 2 Mbaud default).
- Implement ST-Link trace commands:
  - `DBG_APIV2_START_TRACE_RX` (`0xF2 0x40`)
  - `DBG_APIV2_STOP_TRACE_RX` (`0xF2 0x41`)
  - `DBG_APIV2_GET_TRACE_NB` (`0xF2 0x42`)
- Add physical USB Endpoint 0x82 (Bulk/Interrupt IN) to the ST-Link descriptor.
- Support both polled host reads (STM32CubeIDE, OpenOCD) and continuous streaming.
- Implement overrun protection that cleanly advances buffer tail and counts dropped bytes when hosts pause.
- Expose live receiver telemetry via REST (`/api/programmer/swo/status`).

**Non-Goals:**
- On-device packet parsing of ITM/DWT frames (host tools decode the raw NRZ stream).
- Manchester SWO encoding (all modern ARM Cortex-M tools standardise on asynchronous UART/NRZ).

## Decisions

### Decision 1: 4 KiB Circular Ring Buffer with Atomic Running Counts

- **Choice**: Implement a 4096-byte static ring buffer with running 32-bit monotonically increasing head (bytes captured from hardware) and tail (bytes consumed by USB/host) counters.
- **Rationale**:
  - Running monotonic counters simplify wrap calculation and overrun detection without modulo hazards.
  - Matches the verified architecture in `ref/hybridlink/port/stm32f1/hal.c`.

### Decision 2: Strict Packet Pacing on EP 0x82

- **Choice**: In polled mode, after answering `GET_TRACE_NB` with count $N$, the firmware guarantees sending exactly $N$ bytes across EP 0x82 in 64-byte chunks followed by an exact remainder short packet.
- **Rationale**:
  - STM32CubeIDE triggers fatal read errors if a short packet arrives before the full promised count is delivered, or if more bytes are sent than promised.

### Decision 3: Half-Buffer Overrun Strategy

- **Choice**: If unconsumed bytes in the ring buffer exceed `SWO_SIZE - 256` bytes (e.g. while GDB is stopped at a breakpoint), the buffer tail is advanced by half the buffer (`SWO_SIZE / 2`), and the dropped byte counter is incremented.
- **Rationale**:
  - Preserves the freshest trace packets while preventing corrupt wrapping or head/tail pointer inversion.

## Risks / Trade-offs

- **[Risk] Host polling latency during GDB single-stepping**
  - *Mitigation*: Dropped bytes are tracked accurately and reported in telemetry rather than stalling the USB endpoint.
- **[Risk] Endpoint count increase**
  - *Mitigation*: Adding EP 0x82 brings ST-Link mode to 7 IN endpoints, which remains safely below the RP2350 hardware limit of 8 IN endpoints.
