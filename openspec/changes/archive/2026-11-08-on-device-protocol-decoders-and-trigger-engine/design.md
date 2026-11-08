## Context

AgentLA's logic analyzer records raw digital pin transitions via RP2350 PIO state machines and streams RLE-compressed words across DMA buffers. Currently, protocol decoding is performed entirely off-probe in client-side JavaScript or host software (Sigrok / PulseView). While effective for offline inspection, offloading protocol decoding to the host creates high network bandwidth overhead and prevents the probe from reacting autonomously to bus events (such as halting a target core via SWD or freezing capture on an I2C NACK).

This change introduces on-device streaming protocol decoders (Tier 1: UART, I2C, SPI, CAN 2.0A/B) within `crates/agentla-analyzer`, integrated with a real-time Hardware Trigger Matrix that allows protocol-qualified trigger conditions to directly drive hardware actions on the RP2350 with zero heap allocation.

## Goals / Non-Goals

**Goals:**
- Provide zero-heap `#![no_std]` streaming FSM decoders for UART, I2C, SPI, and CAN 2.0A/B inside `crates/agentla-analyzer`.
- Implement a Hardware Trigger Matrix supporting rule predicates on decoded bus fields (e.g. `bus == I2C && addr == 0x48 && ack == false`, `bus == CAN && id == 0x7DF`).
- Wire trigger matching to hardware actions: freeze capture ring buffer, assert external trigger GPIO, dispatch SWD breakpoint/halt to target, and emit low-latency WebSocket alert.
- Expose REST and WebSocket control endpoints in `firmware-rs`:
  - `POST /api/v1/analyzer/decoders/configure`
  - `POST /api/v1/analyzer/triggers/configure`
  - `GET /api/v1/analyzer/stream/decoded` (bidirectional WebSocket)
- Integrate a live protocol inspector table in `webui/lab.html` and `webui/app.js` with instant filtering by bus, address, or ID.
- Maintain strict zero-heap `#![no_std]` compliance and verify memory consumption stays within the 636 KiB Flash / 287 KiB SRAM headroom.

**Non-Goals:**
- Offline full-decode file reprocessing of gigabyte trace dumps (handled by desktop Sigrok/PulseView).
- Tier 2/3 decoders (1-Wire, WS2812B, LIN, SWD sniffer) in this initial phase; they build upon this foundation in a subsequent change.
- Altering the A/B firmware slot layout or LittleFS partition boundaries.

## Decisions

### 1. Ingestion Interface: Transition Stream vs. Raw Clock Samples
- **Decision**: Decoders process a stream of timestamped transition events `(timestamp_ticks: u32, pin_states: u32)` rather than oversampled raw bit arrays.
- **Rationale**: The logic analyzer capture core already produces RLE/Bit-Delta compressed transition streams via PIO. Operating on transitions eliminates redundant loops during idle bus periods and allows decoders to keep pace with microsecond edge transitions without CPU exhaustion.
- **Alternative considered**: Polling raw GPIOs in an interrupt handler. Rejected because jitter at 100+ MHz sampling rates causes dropped bits.

### 2. State Machine Representation: Zero-Heap Enumerations
- **Decision**: Represent all decoder states as explicit Rust enums with bounded array buffers for multi-byte payloads (e.g., `[u8; 8]` for CAN/I2C data frames).
- **Rationale**: Preserves `#![no_std]` invariant, ensures constant-time execution without heap allocation or fragmentation, and guarantees static memory bounds (~12 KiB SRAM total for all 4 decoders).
- **Alternative considered**: Allocating dynamic `heapless::Vec` or trait objects with allocators. Rejected due to strict embedded predictability.

### 3. Hardware Trigger Matrix: Predicate Tree with Compile-Time Array Bounds
- **Decision**: Configure trigger rules as an array of up to 8 trigger descriptors in static storage. Each descriptor specifies:
  - Bus target (UART, I2C, SPI, CAN)
  - Match criteria (address mask, ID mask, error flags, byte pattern match)
  - Action flags (FreezeCapture, AssertTriggerPin, HaltSwdTarget, EmitWsAlert)
- **Rationale**: Evaluating fixed-size struct criteria takes < 50 CPU cycles per decoded event on the 150 MHz Cortex-M33, enabling sub-microsecond trigger latency without stalling DMA transfers.

### 4. Streaming Protocol: Compact JSON over WebSocket
- **Decision**: Stream decoded events as compact JSON objects over `/api/v1/analyzer/stream/decoded` using an SPSC ring buffer.
- **Rationale**: Directly ingestible by browser JavaScript WebUI without requiring proprietary binary parsers, while achieving 100x lower bandwidth than raw logic analyzer waveform streams.

## Risks / Trade-offs

- **[Risk] High-speed SPI clock (e.g. > 10 MHz) could outrun software FSM decoding during sustained bursts.**
  → **Mitigation**: Decoders run on Core 1 or an Embassy async cooperative task fed by a 2048-word DMA ping-pong FIFO. If the FIFO approaches 90% capacity, a `buffer_overrun` flag is flagged in the telemetry stream rather than silently corrupting data.
- **[Risk] Bit-stuffing anomalies on noisy CAN physical buses.**
  → **Mitigation**: CAN decoder validates both hardware CRC-15 and strict bit-stuffing rules; corrupted frames are surfaced as `CanError::BitStuffing` or `CanError::CrcMismatch` events.
- **[Risk] Inter-decoder pin conflict with active pin owners.**
  → **Mitigation**: All decoder channel mappings must be verified and claimed through `pin_owner` / `agentla-pin-arbiter` prior to arming.
