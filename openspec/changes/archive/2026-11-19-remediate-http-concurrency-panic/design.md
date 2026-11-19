## Context

AgentLA runs a monolithic firmware on the RP2350 with 520 KiB total SRAM. The web subsystem in `firmware-rs/src/web.rs` uses `picoserve` to route HTTP and WebSocket traffic over an `embassy-net` TCP stack bound to the CDC-NCM Ethernet gadget interface.

In commit `ea6c86b`, the server concurrency was bumped from 4 to 8 concurrent worker sockets. In `picoserve`, `make_app()` returns a router with 150+ routes, creating a deep nested type `Router<Either<...>>`. Combining 8 `listen_and_serve` futures in a single `join(join4, join4)` inside `web_server_task` creates a static task storage pool of **324.8 KiB**, consuming over 62% of SRAM. During active HTTP traffic, stack frames and header slice accesses trigger a panic inside `slice_index_fail`, which invokes `panic-probe` -> `udf` -> `HardFault`, causing the watchdog to reboot the chip.

## Goals / Non-Goals

**Goals:**
- Reduce HTTP worker concurrency from 8 workers down to 2 workers to slash the task pool size from ~325 KiB down to < 85 KiB, freeing over 240 KiB of SRAM.
- Prevent stack exhaustion by allocating worker buffers (`http_buf`, `tcp_rx_buf`, `tcp_tx_buf`) cleanly without deep call-stack recursion.
- Ensure incoming HTTP connections (e.g. `GET /api/v1/system`, `GET /barefoc`, `GET /`) return proper responses rather than triggering panics and watchdog resets.
- Validate live HTTP communication over USB-NCM with zero dropped packets and zero `ifindex` increments.

**Non-Goals:**
- Replacing `picoserve` with another HTTP server crate.
- Changing API routes, schemas, or WebUI client contracts.
- Altering USB descriptors or USB-NCM device/host MAC addresses.

## Decisions

### Decision: 2 Concurrent Workers via `embassy_futures::join::join`
- **Choice**: Shrink from 8 workers (`join(join4, join4)`) down to 2 workers (`embassy_futures::join::join(server_0, server_1)`).
- **Rationale**: For an embedded instrumentation probe, 2 concurrent HTTP workers easily handle concurrent REST polling and asset streaming without starvation, while dropping memory usage by 75%.
- **Alternatives Considered**:
  - Keep 8 workers by allocating tasks dynamically: Fails due to `#![no_std]` zero-heap constraint.
  - 1 worker: Acceptable for pure sequential requests, but 2 workers allow long-lived SSE/streaming endpoints while still servicing parallel REST status calls.

### Decision: Explicit Buffer Isolation
- **Choice**: Keep separate dedicated buffers for `worker_0` and `worker_1`.
- **Rationale**: Ensures complete spatial independence for each worker socket without mutex contention.

## Risks / Trade-offs

- **[Risk] High-volume browser burst requests (e.g. parallel image/CSS fetching) may experience modest queueing.**
  - *Mitigation*: The browser client caches static assets (`Cache-Control: public, max-age=604800`) and the App Launcher lazily loads assets. 2 workers provide sufficient concurrency over the point-to-point 12 Mbps / 480 Mbps USB interface.
- **[Risk] Long-lived SSE streams blocking worker slots.**
  - *Mitigation*: Streaming endpoints under `/api/v1/*/stream` utilize chunked/SSE writers, and keeping 2 workers ensures at least one slot remains open for command/REST execution.
