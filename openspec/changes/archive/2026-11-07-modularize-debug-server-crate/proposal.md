## Why

The GDB Remote Serial Protocol (RSP) stub (`gdb_server.rs`) and SEGGER RTT circular buffer parser (`rtt.rs`) in `firmware-rs` are currently tied directly to `embassy_net::tcp::TcpSocket`. In Embassy standard architecture, protocol servers should operate over generic `embedded_io_async::Read + Write`. Decoupling them into `crates/agentla-debug-server` allows:
- Testing the entire GDB RSP command parser ($qSupported, $g, $m, breakpoints) and RTT ring buffer search on host desktop with mock byte vectors.
- Running GDB or RTT over any Embassy transport (TCP socket, USB CDC-ACM serial, UART, or WebSocket) without modifying the debug server logic.

## What Changes

- **Add `crates/agentla-debug-server`**:
  - Implements transport-agnostic GDB RSP protocol parser: checksum validation, packet framing (`$...#xx`), ack handling (`+`/`-`), register reads/writes, memory inspection, and breakpoint control.
  - Implements SEGGER RTT control block discovery, ring buffer descriptors, and non-blocking byte extraction over generic memory access traits.
  - Native `#![no_std]` with optional `defmt` and `std` features.
  - Comprehensive host unit tests on `x86_64` verifying GDB RSP command sessions and RTT search.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-debug-server`.
  - Refactors `gdb_server.rs` and `rtt.rs` to delegate protocol parsing to `agentla-debug-server` while passing Embassy TCP sockets.

## Capabilities

### New Capabilities
- `domain-crate-debug-server`: Transport-agnostic GDB Remote Serial Protocol stub and SEGGER RTT ring buffer engine.

### Modified Capabilities
<!-- None: External GDB client and RTT client behavior remains 100% compliant. -->

## Impact

- **Codebase**: `firmware-rs/src/programmer/{gdb_server, rtt}.rs` delegate to `agentla-debug-server`.
- **Cargo Workspace**: `crates/agentla-debug-server` implemented with host tests.
