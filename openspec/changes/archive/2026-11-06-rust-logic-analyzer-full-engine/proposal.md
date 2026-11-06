## Why

The logic analyzer is AgentLA's signature feature. The Rust port only ships a state-machine stub (Phase 3 scaffold); the C implementation (`firmware/src/analyzer/*` + `capture.pio`) provides a 1.2 M sample-capacity bank, RLE compression, 11 protocol decoders, and trigger-qualified capture that is entirely unavailable on the Rust branch. Users requiring any logic-analysis capability today must use the older C build — the Rust bridge cannot demonstrate the project's core competency without completing this port.

## What Changes

- **PIO capture program family** port: bring all `capture.pio` programs (`BLAST_CAPTURE`, `POSITIVE_CAPTURE`, `NEGATIVE_CAPTURE`, `COMPLEX_CAPTURE`, `FAST_CAPTURE`, `NO_TRIGGER_CAPTURE`, `pio_rle_capture`, `pio_trigger_qualifier`) under embassy-rp's `Pio`/`FlexPio` types with runtime program re-instantiation (only one active SM at a time).
- **DMA circular buffer ring**: capture data is funneled from the SM FIFO into SRAM-static circular banks sized for 416 KB (SCOPE) / 64 KiB (STREAM) / 16 KiB (RLE), with `dma::Transfer` completion interrupts feeding the async driver loop.
- **RLE encoder + SSE stream**: `RunLength:16 | PinState:16` words streamed over the existing `/api/v1/events` channel and buffered for synchronous `/api/v1/logic-analyzer/data` pagination.
- **Trigger subsystem**: single rising/falling edges, any-pin pattern values (inline trigger qualifier state machine), pre-trigger percent (default 50, pre-size 64 KiB), and complex multi-pair qualification; no-trigger (immediate) burst.
- **State REST surface**: `/api/v1/logic-analyzer/{config,status,start,stop,data,triggers}` mapped per the existing `rest-logic-analyzer` spec.
- **Pin bank claim**: the capture bank claims contiguous flex-pin spans via the Phase-2 `pin_owner` registry before arming, ensuring there is no overlap with servo/adapter claims.
- **11 protocol decoders port** as standalone `deerders::...` crates: SWD, SWO, CAN, LIN, I3C, 1-Wire, Modbus, DMX512, DMX, SPI Flash, LED.
- **Memory accounting and safety**: static BSS bank below 520 KB; `DMA guard` pointer in I/O registers; graceful `HTTP 503` for oversized banks.
- **Behavior parity**: all C start commands documented in `firmware/src/analyzer/capture.h` → Rust function names.

## Capabilities

### New Capabilities
- `pio-program-servant`: PIO program bootstrap (reuse existing `programmer/pio_programs.rs`, expanded with capture mode set), state-machine setup with consistent wrap-target behavior, dynamic clock dividers.
- `capture-dma-streaming-ring`: circular buffer ring implementation with DMA completion-interrupt handlers and async task loop.
- `rle-on-wire-compression`: ex-hardware RLE format + host-side decode for the SSE/web.
- `trigger-qualifier-state`: multi-pin pattern triggers with pre-trigger percent and hardware-IRQQual.
- `proto-decoder-frame`: standalone decoders (host side integration, not part of MCU capture — passive).
- `logic-analyzer-endpoints`: REST endpoints per spec.

### Modified Capabilities
- `pin-ownership`: Adds `capture-bank` claims consuming `CAP_PIO` flexible pins; `CaptureBank` registry interface.
- `rest-logic-analyzer`: ports all endpoints to Rust web server with C-parity shapes.

## Impact

- New Rust modules: `firmware-rs/src/analyzer/{capture,dma,rle,trigger,decoders::*}`, `pio_programs.rs` extended.
- Flash: +~100 KB (capture code); SRAM: 416 KB allocation from static BSS, bumped toward 470 KB of 520 KB — verify in build budget.
- Dependencies: `embassy-rp` needs `pio`/`dma` feature enables (already compiled with these in other clones; this re-uses them); `embedded-alloc` maybe skipped if we keep static buffers.
- Hardware: enables PIO0 `sm0` capture bank choice; no pin changes beyond flex-pool parity.
- Runtime cost: DMA completion interrupt at up to 100 MHz; watch psrest (async task react time) flagged for hardware testing.