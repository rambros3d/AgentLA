## 1. Protocol Decoder Core Types & Architecture

- [x] 1.1 Create `crates/agentla-analyzer/src/decoder/mod.rs` with `ProtocolDecoder` trait, `ProtocolKind`, and error enums
- [x] 1.2 Implement zero-heap `DecodedEvent` enum and timestamped transition representation in `crates/agentla-analyzer/src/decoder/event.rs`

## 2. Tier 1 Streaming Decoders Implementation

- [x] 2.1 Implement `crates/agentla-analyzer/src/decoder/uart.rs` (auto-baud estimation, 5–9 data bits, parity checking, framing/break errors)
- [x] 2.2 Implement `crates/agentla-analyzer/src/decoder/i2c.rs` (START/STOP detection, 7/10-bit addressing, R/W flag, ACK/NACK, SMBus PEC CRC-8)
- [x] 2.3 Implement `crates/agentla-analyzer/src/decoder/spi.rs` (CPOL/CPHA Modes 0–3, active-low/high CS, full-duplex MOSI/MISO byte assembly)
- [x] 2.4 Implement `crates/agentla-analyzer/src/decoder/can.rs` (standard 11-bit & extended 29-bit ID, RTR, DLC, CRC-15 check, bit-stuffing verification)
- [x] 2.5 Add unit test suite in `crates/agentla-analyzer/tests/decoder_tests.rs` verifying all 4 decoders against synthetic transition vectors

## 3. Hardware Trigger Matrix & Dispatcher

- [x] 3.1 Implement `crates/agentla-analyzer/src/decoder/matrix.rs` with static 8-rule predicate table and rule evaluation engine
- [x] 3.2 Wire trigger match actions: capture buffer freeze, external trigger pin assertion, SWD target halt signal, and WebSocket alert dispatch
- [x] 3.3 Link protocol trigger output signal into `firmware-rs/src/analyzer.rs` capture state machine

## 4. Firmware REST & WebSocket Control Surface

- [x] 4.1 Implement `POST /api/v1/analyzer/decoders/configure` in `firmware-rs/src/rest.rs` with pin arbiter verification
- [x] 4.2 Implement `POST /api/v1/analyzer/triggers/configure` in `firmware-rs/src/rest.rs` to install rule predicates
- [x] 4.3 Implement WebSocket streaming endpoint `GET /api/v1/analyzer/stream/decoded` in `firmware-rs/src/fs_http.rs` streaming compact JSON events
- [x] 4.4 Expose decoder configuration and active trigger state in `GET /api/v1/analyzer/status`

## 5. WebUI Protocol Inspector & Trigger Builder

- [x] 5.1 Add Protocol Inspector table component and bus filter pills (ALL, UART, I2C, SPI, CAN) to `webui/lab.html`
- [x] 5.2 Add Trigger Rule builder dialog in `webui/lab.html` (select bus, address/ID condition, choose actions)
- [x] 5.3 Wire real-time `/api/v1/analyzer/stream/decoded` WebSocket stream into `webui/app.js` with auto-scrolling log table

## 6. Integration Verification & Flash/RAM Auditing

- [x] 6.1 Create `test/test_protocol_decoders.py` covering decoder configuration, trigger rule firing, and WebSocket frame delivery
- [x] 6.2 Reduce OTA slot size from 1.2 MB to 1 MB (1,024 KiB) and audit release firmware footprint (comfortably within 1,024 KiB)
