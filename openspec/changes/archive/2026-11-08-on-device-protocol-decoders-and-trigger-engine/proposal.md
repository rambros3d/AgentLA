## Why

Currently, AgentLA's logic analyzer records raw digital pin transitions via PIO/DMA, compresses them with RLE/Bit-Delta, and offloads protocol decoding entirely to the client browser or host sigrok engine. While this works for post-mortem visual waveform inspection, it creates two fundamental engineering bottlenecks:
1. High-speed digital transitions require streaming megabytes of raw data over USB/Ethernet to the browser, exhausting network bandwidth on high sample-rate captures.
2. The probe lacks real-time awareness of bus semantics, making it impossible to trigger hardware actions (freezing capture buffers, asserting target nRESET, or breaking an MCU core via SWD) on specific protocol events such as an I2C NACK on device address 0x48, a UART panic sequence, or a CAN error frame.

Implementing on-device streaming protocol decoders (Tier 1: UART, I2C, SPI, CAN) coupled with a real-time Hardware Trigger Matrix empowers AgentLA to inspect, filter, and trigger on embedded bus events directly on the RP2350 with sub-microsecond latency and zero-copy `#![no_std]` efficiency.

## What Changes

- Implement zero-heap `#![no_std]` streaming FSM decoders in `crates/agentla-analyzer` for:
  - **UART / Async Serial**: Auto-baud estimation, 5-9 data bits, parity checking, framing and break detection.
  - **I2C / SMBus**: START / Repeated START / STOP FSM, 7-bit & 10-bit addressing, R/W flag, ACK/NACK validation, clock stretching watchdog, and PEC CRC-8 verification.
  - **SPI**: Modes 0, 1, 2, 3, active-low/high Chip Select framing, full-duplex synchronous MOSI/MISO byte packing.
  - **CAN 2.0A / 2.0B**: Standard & Extended ID parsing, RTR, DLC, CRC-15 verification, and bit stuffing error detection.
- Implement the **Protocol Hardware Trigger Matrix**:
  - Programmable trigger condition matching (`bus == I2C && addr == 0x48 && ack == false`, `bus == CAN && id == 0x7DF && data[0] == 0x02`).
  - Real-time hardware action dispatcher: freeze capture ring buffer, assert trigger pin, halt target via SWD, or dispatch immediate alert WebSocket packet.
- Expose REST and WebSocket endpoints in `firmware-rs`:
  - `POST /api/v1/analyzer/decoders/configure`: Configure active decoder channels and parameters.
  - `POST /api/v1/analyzer/triggers/configure`: Install protocol trigger rules into the hardware engine.
  - `GET /api/v1/analyzer/stream/decoded`: Bidirectional WebSocket streaming decoded protocol event objects (`{"t": 12.4, "bus": "i2c", "event": "ack_fail", ...}`) with 100x lower bandwidth than raw waveform data.
- Integrate WebUI live protocol inspector table in `webui/lab.html` and `webui/app.js` with instant filter by bus, address, or ID.

## Capabilities

### New Capabilities
- `on-device-protocol-decoders`: Streaming FSM protocol decoders (UART, I2C, SPI, CAN 2.0A/B) executing in `#![no_std]` on the RP2350 with zero heap allocation.
- `protocol-hardware-trigger-matrix`: Real-time trigger evaluation engine matching decoded protocol packets against configurable rule predicates to drive physical probe actions (freeze capture, assert GPIO trigger out, halt target via SWD).

### Modified Capabilities
- `logic-analyzer-capture-core`: Feed double-buffered DMA transition samples directly into active protocol decoder state machines and link trigger flags to the capture sequencer.
- `rest-logic-analyzer`: Extend REST and WebSocket control surface to configure on-device decoders, upload trigger predicates, and stream decoded packet streams.

## Impact

- **Crates**:
  - `crates/agentla-analyzer`: New `decoder` module (`uart`, `i2c`, `spi`, `can`, `matrix`).
  - `firmware-rs`: REST route handlers and WebSocket frame dispatcher in `rest.rs` and `analyzer.rs`.
- **Memory & Storage**:
  - Consumes ~32 KiB Flash out of 636 KiB available headroom in Firmware Slot A.
  - Consumes ~12 KiB SRAM out of 287 KiB unallocated memory for streaming FIFOs.
  - Zero disruption to LittleFS partitions (`SYSTEM_FS` and `USER_FS`) or dual-slot A/B OTA bootloader.
