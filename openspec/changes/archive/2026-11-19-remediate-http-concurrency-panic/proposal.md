## Why

Connecting to TCP port 80 on AgentLA currently causes a firmware panic and an immediate MCU reboot on the RP2350. Diagnostic SWD inspection confirmed an escalated UsageFault/HardFault triggered by `panic-probe` (`rust_begin_unwind`) during HTTP worker connection handling.

The crash occurs because `web_server_task` joins 8 concurrent `picoserve::Server` instances in a single async task via `join(join4, join4)`. This inflates the task's future storage pool in SRAM to **324.8 KiB** (over 62% of the RP2350's total 520 KiB RAM), monomorphizing over 150 nested route handlers 8 times over and overflowing call stacks or slice bounds during header parsing. Reducing worker concurrency to a bounded pool (e.g. 2 workers) and ensuring robust slice bounds in header iteration prevents stack exhaustion and restores reliable HTTP and WebUI connectivity.

## What Changes

- Reduce the concurrent HTTP worker pool in `web_server_task` from 8 workers down to 2 bounded worker sockets, dramatically shrinking task future memory footprint from ~325 KiB down to < 85 KiB.
- Allocate HTTP worker buffers in static cells or dedicated task contexts to avoid massive stack frame allocations inside async poll handlers.
- Harden HTTP request parsing and header iteration against out-of-bounds slice accesses that trigger `slice_index_fail` panics.
- Verify end-to-end HTTP request/response flow over USB-NCM (`http://10.10.10.10/api/v1/system` and `/barefoc`) with hardware SWD logging to confirm zero panics.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `network-access`: Bounded HTTP worker pool concurrency and non-crashing TCP port 80 request processing under Embassy.
- `url-routing-engine`: Panic-free header parsing and request dispatch across canonical and REST routes.

## Impact

- `firmware-rs/src/web.rs`: Worker task instantiation, buffer allocation, and concurrency joins.
- `firmware-rs/src/fs_http.rs`: Request header parsing and chunk streaming handlers.
- Host USB-NCM link stability: USB interface resets (`ifindex` increments) caused by HTTP watchdog reboot cycles are eliminated.
