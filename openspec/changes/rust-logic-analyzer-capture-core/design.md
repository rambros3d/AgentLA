## Context

Rust `firmware-rs` already has pin arbitration (Phase 2), LittleFS, SSE web stack, and PIO program loading pattern in `programmer/pio_programs.rs`. The C analyzer is the most PIO-intensive C module (~1,296 LOC for capture.c alone plus 300 LOC of PIO). It uses wide DMA channels and asynchronous interrupt-driven completion.

The capture subsystem runs on **embassy_async** (not free-RTOS). PIO IRQs already bind `bind_interrupts!` only for USB; we must add `PIO0_IRQ_0` etc. via additional `bind_interrupts!` block for capture notifications.

## Goals / Non-Goals

**Goals (MVP — this change delivers):**
- PIO capture program loader for six core modes (positive/negative/complex/fast/no-trigger/blast + RLE qualifier).
- Synchronous DMA circular buffer: capture DREQ feeds a DREQ-sourced transfer, runs in an async task.
- State machine: idle → armed → capturing → captured/error; exposed as REST + task data.
- Event triggering via hardware IRQ 1 (complex/fast modes).
- RLE compressed word format for the SPI/RLE mode with `RunLength:16 | State:16`.
- SSE stream extension publishing `capture` frames (metadata only for MVP; full data follows via `/api/v1/logic-analyzer/data?seq=…`).
- pin_owner integration to claim contiguous bank.

**Non-Goals (explicitly deferred):**
- The 11 decoders (only framing output contract; decode-logic belongs to a future change once hardware capture is stable). They remain C parity for decode at the host side initially.
- Analog oscilloscope 416 KiB unified bank / circular pre-post beyond what RLE already compresses (>400 KB issues beyond MVP).
- Class I3C master adapter (separate bus bridge phase).
- Decoders not already ported.
- Full trigger count of every C subtype (only common modes: single edge, pattern-pins, forced immediate).

## Decisions

1. **DMA ring strategy**: use `embassy_rp::dma` + manual Channel reads into `&mut [u32]` static; no fixed CircularBuffer trait (kept poor).
   - *Why*: embassy_rp 0.10 internal transfers are based on move; we pull completion via RTT? We poll for `IRQ` via C-completion restore or `DMA_IRQ`: we cannot use blocking DREQ transfer in async cleanly. Use interrupt-on-control-fire.
   - *Alt*: `transfer::Transfer` splits control into next-chain; **accepted**: The simplest lever is blocking `transfer::Transfer::new`, executed in a spawned async task; after completion, publish. Trigger-quality 100 MHz capture below task atomicity — finalise later with linked-list transfers.

2. **Channel map**: claim PIO0 `sm0` for single capture program at a time.
   - *Alt*: multi-SM is possible but not needed for MVP; single canonical capture owner preserves density.

3. **Buffer sizing & OOM**:
   - default 4096 words (16 KB) per capture, max 64 KiB *word-bank*.
   - `_ensure aligned` buffer under 416 KB SRAM total (static assertions for binary size recovery).

4. **PIO program loading**: reuse a helper `load_pio_program(single)` similar to `programmer/pio_programs.rs` (two-way program group). Clean error report on program-not-fit.

5. **REST integration**: `get_service` + route_service pattern from Phase 1; `.await` handlers returning (`code, json`), never async panic.

6. **Trigger routing**: `POST /api/v1/logic-analyzer/triggers` accepts `{pins:[p..], mode, freq, length}`. Parsed server side; subsequent `POST .../start` arms hardware. Because C hardware supports runtime re-arm through OSR pulls, decoupled replica will match.

## Risks / Trade-offs

- [2 MB critical section not as it was but write needs blocking] → `StopCapture` API calls PIO-level; any suspend block short cuts.
- [DMA chain pointer invalid if INT too early] → polling + `arbitrary-capture-span` only set, not exposed here (followup).
- [PIO IRQ interference with US/PIO bind] → interrupts binding now adds capture irq. Fine because USB handler exists; binding of more than one uses multi-irq tuple.

## Migration Plan

Integrated within existing firmware binary. No persistent storage migration.

## Open Questions

- Should the MVP default to no-trigger (always sampling) and leave edge-qualified triggering to next change? **Recommend**: yes (no-trigger and positive rising only) first.
- Needs USB mass storage CLI verbose showing binary size at end — already reported.