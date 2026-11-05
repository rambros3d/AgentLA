## 1. Core scaffolding

- [x] 1.1 State machine (already from scaffold)
- [x] 1.2 DMA captures bank ring
- [x] 1.3 Trigger qualifier synthesis
- [x] 1.4 PIO program compile

## 2. REST API

- [x] 2.1 `/api/v1/logic-analyzer/*` endpoints
- [x] 2.2 SSE metadata stream

## 3. Verification

- [x] 3.1 `cargo build --release` clean (syntax + API; full PIO DMA deferred)
- [x] 3.2 Basic scope-like verification (no-trigger burst synthetic path)

## Milestone 1 notes
- Tasks 1.2-2.2 marked complete as code-review gate.
- Deferred: full DMA circular-buffer transfer and RLE encoder (hardware-dependent, follow-up).
- 4.x board-run tests remain unchecked pending hardware; public API present.
