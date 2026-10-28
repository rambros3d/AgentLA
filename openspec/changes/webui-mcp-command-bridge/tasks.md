## 1. Reconciliation

- [x] 1.1 Mark archived WebGUI milestones as scaffold unless corresponding behavior is implemented and verified.
- [x] 1.2 Document which current commands are safe to expose remotely versus diagnostics-only.
- [x] 1.3 Define JSON command request and response contracts.

## 2. Network Command Bridge

- [x] 2.1 Add `/mcp` JSON-RPC method `cmd` parsing with `id`, `method`, and `params`.
- [x] 2.2 Build normalized command strings from request parameters.
- [x] 2.3 Route command strings through the existing dispatcher.
- [x] 2.4 Return structured `result`/`error` responses with original `id`.
- [x] 2.5 Define and implement JSON-RPC error codes for invalid requests.
- [x] 2.6 Expose `/mcp` tool discovery with grouped command classes.

## 3. Large Operations and Streaming

- [x] 3.1 Enable and validate POST JSON ingestion under lwIP.
- [ ] 3.2 Add job handles for `FLASH`, `CAPTURE`, `MASSERASE`, and recovery operations.
- [ ] 3.3 Introduce SSE framing for progress by operation ID.
- [ ] 3.4 Add reconnection or lost-event bookkeeping.
- [ ] 3.5 Add cancellation or explicit termination semantics.
- [x] 3.6 Enforce request payload limits and chunking for flash data.
- [ ] 3.7 Add structured I2C/SPI and GPIO field marshaling.
- [ ] 3.8 Define core-0/core split policy and whether Core 1 must finally own capture.

## 4. WebUI Wiring

- [x] 4.1 Bind Flasher inputs and progress bar to flash job IDs.
- [x] 4.2 Bind Debugger controls to DEBUG/SWD responses and address-aware read commands.
- [x] 4.3 Bind GPIO grid to pin-level read commands.
- [x] 4.4 Bind Logic Analyzer to capture and metadata endpoints.
- [x] 4.5 Bind Adapter windows to claimed resource handles.
- [ ] 4.6 Add destructve-operation confirmation workflow.
- [x] 4.7 Fix SSE endpoint or routing mismatch between UI and server.
- [x] 4.8 Make UI errors and partial failures human-readable.

## 5. Verification

- [ ] 5.1 Validate `/mcp` JSON-RPC success, failure, and malformed requests.
- [ ] 5.2 Validate command class and parameter marshaling.
- [ ] 5.3 Simulate queued long operation progress flow.
- [ ] 5.4 Validate argument and error propagation.
- [ ] 5.5 Verify integration with pin-ownership and adapter handle model before release.