## 1. Cad specification (done, see specs)

- [x] 1.1 Outline API surface for 1st pass (done above)

## 2. Scaffolding

- [x] 2.1 Create `firmware-rs/src/analyzer.rs` module (state machine Idle/Armed/Capturing/Captured/Error/Stopped)
- [x] 2.2 Define `CaptureConfig { freq, length, channels, trigger, pre_pct }` + ring buffer
- [x] 2.3 Export methods: `arm`, `start`, `stop`, `status`
- [x] 2.4 Wire mod + init in `main.rs`

## 3. Web Surface (REST + SSE)

- [x] 3.1 `/api/v1/logic-analyzer`: cmd/start, stop, config update, status, data chunk
- [x] 3.2 SSE `capture` event per bank burst (metadata only)

## 4. Verification

- [x] 4.1 `cargo build --release` clean, 0 new warnings
- [x] 4.2 HTTP smoke test (ping + status 200 via `/system` etc.)
