## Why

AgentLA already has a working PIO/DMA capture engine, RLE compression, and 11 decoders in the C firmware (`firmware/src/analyzer/*` + `firmware/pio/capture.pio`), but the Rust `firmware-rs` port ships none of it — the logic-analyzer half of the device can only be used from the C build. Pin arbitration (Phase 2) is now in place so the capture bank can claim its contiguous trigger-plus-data pin span safely, removing the last blocker before porting.

## What Changes

- **Capture engine core (Rust port)** — implement Mode A (in-hardware PIO RLE streaming), Mode C (circular pre/post-trigger oscilloscope bank), and no-trigger immediate capture; multi-pin span capture via embassy PIO with up to 32-bit width; RLE-compressed word stream; buffer ring driven over SSE for streaming to host (`/api/v1/events` extension).
- **PIO programs** ported from `capture.pio`: `BLAST_CAPTURE`, `POSITIVE_CAPTURE`, `NEGATIVE_CAPTURE`, `COMPLEX_CAPTURE`, `FAST_CAPTURE`, `NO_TRIGGER_CAPTURE`, `pio_rle_capture`, `pio_trigger_qualifier`.
- **Trigger subsystem** — simple edge triggers (rising/falling per pin), pattern/value triggers spanning several trigger pins, configurable pre-trigger percent (default 50%, pre-mask 64 KiB of a 416 KiB bank), post-trigger sample count, complex inline assembly multi-cycle qualifying states.
- **REST surface for analyzer**: `/api/v1/logic-analyzer` command interface (`/start`, `/stop`, `/config`, `/status`, `/data`) for SRAM/local-capture inspection and SSE streaming — endpoints parity per `openspec/specs/rest-logic-analyzer`.
- **pin_owner integration**: capture bank claims (GPIO0–29 range via flex-capability in Phase 2 pin registry, 31 trigger/config trigger states and bank counters).
- **RLE encoding** port (`rle_util.c` parity): run-count packed 16-bit run length + 16-bit sample.
- Explicit scope creep guard: all 11 protocol decoders + visual-structure decoders are deferred to a follow-up spec (file-decoder packages beyond I/O framing are now C-dependency-free enough to host-test separately), alongside the DMA-subsystem tuning.

## Capabilities

### New Capabilities
- `panic-free-pio-capture-stream` (spec refinement not yet): first capture core needs closer ABI — see `capture.pio` + `capture.h` for spec conformance when finalized.
- `logic-analyzer-capture-commands`: capture begin/stop/status REST + SSE data stream.
- `analyzer-trigger-qualifier`: level-edge-pattern trigger setup and qualifying state machine.

### Modified Capabilities
- `pin-ownership`: makes `capture_bank_claim` semantics claim span over flex pins under a CAP_PIO claim; capture-claim aggregator is now referenced here. `capture-bank-layout` spec remains authoritative for AREA layout.

## Impact

- New Rust modules: `firmware-rs/src/analyzer/` (capture core + RLE + trigger + state machine) and `firmware-rs/src/fs_http.rs` extended with LV/ACK preview; PIO bootstrap similar to `programmer/pio_programs.rs`.
- Flash/RAM: 416 KB bank allocation from static BSS (SRAM usage rises near 463 KB of 520 KB; triggers an intentional SRAM parity with C build). No flash space change.
- Library requirements: embassy-rp `dma` feature + `pio` enabled (already feature flags partly; `rp-pac`; no new deps).
- Async runtime: long-lived task spanning web_server and analyzer; cooperative timeout discipline from Phase 1 still applies.
- Defensive OOM guard: capture stops or reduces buffer if SRAM accounting would encroach `cyw43`/USB guard.