## Context

`firmware-rs` provides an embedded network GDB server (port 3333) and SEGGER RTT telemetry (port 19021). The GDB server converts GDB RSP packets into ARM Cortex-M debug register reads/writes and memory accesses. The RTT reader searches target RAM for the `"SEGGER RTT"` control block and extracts ring buffer payloads.

This design extracts both protocols into `crates/agentla-debug-server` operating over `embedded_io_async::{Read, Write}`.

## Goals / Non-Goals

**Goals:**
- Extract GDB RSP packet framing, checksums, register encoding/decoding, and command dispatch into `agentla-debug-server::gdb`.
- Extract SEGGER RTT control block detection, buffer header parsing, and read/write offset pointers into `agentla-debug-server::rtt`.
- Operate over generic `embedded_io_async::{Read, Write}`.
- Host unit tests verifying full GDB session exchanges on `x86_64`.

**Non-Goals:**
- Managing TCP listeners or IP stacks (those remain in `firmware-rs::net`).
