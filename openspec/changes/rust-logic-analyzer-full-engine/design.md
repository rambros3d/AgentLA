## Context

Firmware-rs already supplies:
- `embassy-rp` Pio glue in `programmer/pio_programs.rs` (load/run/SM reset, same API surface needed by capture).
- Phase 2 pin owner registry (claims, release, quiesce) — capture bank guard derived from flat capture bank claim helper that asserts CAP_PIO.
- Embedded SRAM budget: only 520 KiB for everything; capture bank eats 416 KiB @ SCOPE.
- Atomic/critical regions are ` embassy_executor::Executor` single-core exception tasks.

## Goals / Non-Goals

**Core goals (this change's MVP)**:
- Capturing N bytes on PIO0 SM0 into the static bank, triggered by logic-level edge/pattern.
- Async state model with no blocking other than read of pins.
- Delivery of RLE word frames with prefetch semantics through SSE events only.
**Non-goals**:
- all protocol decoders (separate work item) and 416 KiB allocation tuning; advanced sequencing only header accessor provided.

## Decisions

1. **PIO program loading**: we create a Rust module `capture_pio.rs` exposing one helper `ensure_pio_program(program)` like in Phase2 with `Instance: Pio` parameters but returning `Result<(), StartError>`.
2. **DMA transfer**: Use `embassy_rp::dma::transfer::Transfer` with `.wait()` to block the spawned task. SM uses `push` on completion. DMA size = `ceil(num_samples / u32 bits per sample)` pack words, count ≤ 65536.
3. **Triggering**: `RISING_EDGE`/`FALLING_EDGE` run from separate PIO `SM` qualifies state but uses same IRQ fire; because trigger qualifier and capture SM would interlock, **sequential approach**.
   - *Alt*: multiple `PIO` SM available in RP2350; run # qual SM plus capture SM; heavy for 1st pass. Use trigger qualification in `assembly.S`? not needed now.
4. **Consistency**: use round 3 `CYW` / USB bank counters. 
5. **RLE stream format** same as C (rateword). Exposed to Phase1 SSE channel `capture_stream` max latency 60 ms.
6. **Pin ownership**: `is_flex && CAP_PIO` required (pins drone from 0..29). Full bank occupying always priority claim exclusion.

## Risks / Trade-offs

- [PIO completion deadlock] → `dma.wait()` may block forever if trigger doesn't fire; `StopCapture` uses PAZنتج — but missing; mitigate with watchdog 5s and task abort; clear via state change. Emergency CLI `CAP-stop` added.
- [SSE starvation] → capture task runs priority `Async` only after event loop resumption; add 10ms cooperative delay.
- [Static BSS allocation] 416 KB ring + other 44 KB => RAM ~460/520KB; compiled binary reported to preserve margin; link check required.
- [`DMA transfer::waiting`] may sell outbound if IRQ missed due channel error → use transfer `ControlBlock` on static world.
- [SIMD two-phase sampling] all captured `in pins` pass at no more than 2 cycles leak: wrap `in pins,XX]`.

## Migration Plan

Integrated inside existing firmware binary with optional REST classes. Rollback by revert.

## Open Questions

- Should capture bank claim be in `rest_async` only and withdraw pins if config invalid — yes.
- Should position of `STOPPING/ERROR` map directly to REST errors — recommend 503 on oversized/overflow bank.