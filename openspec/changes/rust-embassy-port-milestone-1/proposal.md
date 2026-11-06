## Why

AgentLA's current C/C++ firmware (~25k LOC) relies on a cooperative polling loop in `main.c` alongside TinyUSB and lwIP. As capabilities expand (autonomous flashing, high-speed logic analyzer, web desktop, and bus bridges), cooperative polling introduces latency jitter, potential memory safety pitfalls, and complex state management across dual cores.

Porting AgentLA to a modern asynchronous Rust stack using Embassy (`embassy-rp`) on the Raspberry Pi RP2350 (Cortex-M33) provides memory safety, structured async/await tasks, compile-time pin ownership semantics, and superior concurrency. Milestone 1 establishes the clean-slate foundation: a standalone git worktree on branch `rust` featuring an RP2350 Embassy runtime, composite USB (Dual CDC, CMSIS-DAP v1/v2, CDC-NCM network), and an asynchronous web server (`picoserve` + `embassy-net` + `littlefs2`).

## What Changes

- Initialize a dedicated git worktree on branch `rust` for greenfield Rust firmware development.
- Target the RP2350 (ARM Cortex-M33 / `thumbv8m.main-none-eabihf`) as the primary architecture with `embassy-rp`.
- Implement an `embassy-usb` composite device:
  - CDC-ACM Port 0: Interactive CLI / REPL.
  - CDC-ACM Port 1: Target UART bridge pass-through.
  - CMSIS-DAP: v1 (USB HID) and v2 (WinUSB bulk endpoints) interface scaffolding.
  - Network: USB CDC-NCM interface for high-throughput virtual networking across Linux, macOS, and Windows 11.
- Implement an `embassy-net` async network stack:
  - Static IP assignment (`10.10.10.10/24`).
  - Embedded DHCP server task offering leases on `10.10.10.11-13`.
  - Captive portal / DNS resolver for `agentla.local`.
- Integrate `picoserve` async HTTP server backed by `littlefs2`:
  - Static web asset streaming from on-flash LittleFS partition.
  - REST API endpoint dispatcher (`/api/v1/system`, `/api/v1/pins`, `/api/v1/jobs`).
  - Server-Sent Events (SSE) telemetry broadcast stream.

## Capabilities

### New Capabilities
- `rust-embassy-runtime`: RP2350 Embassy async runtime, dual-core executor scaffolding, clock initialization, and watchdog supervisor.
- `rust-composite-usb`: Composite USB device with Dual CDC-ACM, CMSIS-DAP v1/v2 endpoints, and CDC-NCM networking.
- `rust-network-web`: `embassy-net` TCP/IP stack with DHCP server, captive DNS, and `picoserve` async HTTP/REST server backed by `littlefs2`.

### Modified Capabilities
None. (This greenfield implementation runs in an isolated worktree and does not alter existing C/C++ firmware specs or behaviors).

## Impact

- **Repository**: Adds a new worktree/branch `rust` containing Cargo workspace `firmware-rs`. The existing CMake build and `firmware/` C tree remain untouched.
- **Hardware Targets**: Targets Raspberry Pi RP2350 (Pico 2, custom RP2350 boards).
- **Host Drivers**: Utilizes standard USB CDC-ACM, CDC-NCM, and WinUSB / HID classes; driverless on modern OSes.
