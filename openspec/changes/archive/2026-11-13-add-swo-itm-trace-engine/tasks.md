## 1. SWO Core Engine & Ring Buffer

- [x] 1.1 Implement `crates/agentla-programmer/src/swo.rs` with 4096-byte circular ring buffer, atomic running head/tail counters, and half-buffer overrun drop logic.
- [x] 1.2 Implement `SwoTelemetry` tracking active status, baud rate, bytes received, bytes delivered, and bytes dropped.
- [x] 1.3 Update `crates/agentla-programmer/src/stlink.rs` to route `DBG_APIV2_START_TRACE_RX`, `DBG_APIV2_STOP_TRACE_RX`, and `DBG_APIV2_GET_TRACE_NB` to the SWO engine.
- [x] 1.4 Add unit tests in `crates/agentla-programmer` covering write/read wrapping, exact byte promises, and overrun handling.

## 2. Hardware Trace Endpoint & Streaming Task

- [x] 2.1 Update `firmware-rs/src/usb.rs` ST-Link descriptor to add physical trace endpoint EP 0x82 (Bulk/Interrupt IN, 64-byte).
- [x] 2.2 Implement `swo_trace_task` in `firmware-rs/src/usb.rs` to stream packets on EP 0x82 adhering to promised counts and short-packet pacing.
- [x] 2.3 Connect hardware UART/PIO receiver on designated SWO GPIO pin to push incoming bytes into the SWO buffer.

## 3. Telemetry & Web Dashboard

- [x] 3.1 Add `GET /api/programmer/swo/status` endpoint in `firmware-rs/src/rest.rs` returning JSON telemetry.
- [x] 3.2 Add live SWO trace health and throughput indicator in `webui/index.html` and `webui/app.js`.

## 4. Verification & Testing

- [x] 4.1 Run unit tests with `cargo test --target x86_64-unknown-linux-gnu -p agentla-programmer`.
- [x] 4.2 Verify embedded build with `cargo check --target thumbv8m.main-none-eabihf`.
