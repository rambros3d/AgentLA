## Why

While AgentLA provides a modern Web Desktop OS, HTTP REST API, and Model Context Protocol (MCP) tool surface, modern embedded developers and CI pipelines rely on industry-standard host IDEs and tools—including VS Code (`cortex-debug`), Keil MDK, CLion, `probe-rs`, OpenOCD, and native GDB. Additionally, commercial probes such as SEGGER J-Link, NXP MCU-Link, and Black Magic Probe have established powerful capabilities like zero-pin real-time logging (RTT), network GDB servers, concurrent double-buffered turbo flashing, and silicon-level unprotect (Nordic CTRL-AP).

Implementing these architectures transforms AgentLA into a universal, high-performance debug probe with 100% plug-and-play compatibility across all existing embedded toolchains, while retaining its unique networked and multi-instrument capabilities.

## What Changes

- **CMSIS-DAP v2 USB Bulk Interface**: Upgrades CMSIS-DAP support to v2 (WinUSB bulk endpoints) alongside v1 HID, providing zero-polling-latency driverless operation on Windows/Linux/macOS for `probe-rs`, OpenOCD, pyOCD, Keil, and VS Code.
- **Segger RTT Background Engine**: Implements background SWD memory scanning for the `"SEGGER RTT"` control block and non-intrusively drains lock-free target SRAM ring buffers while the CPU runs at full speed, streaming logs directly to the Web Desktop Terminal app and WebSocket.
- **On-Probe Network GDB Server**: Listens on `tcp://10.10.10.10:3333` via `embassy-net`, allowing developers to attach directly using `arm-none-eabi-gdb` without running OpenOCD or local debug daemons on their PC.
- **Concurrent Double-Buffered Turbo Flashing**: Upgrades `FlashAlgorithmRunner` to allocate ping-pong SRAM buffers in the target, streaming chunk $N+1$ via SWD DMA concurrently while the target CPU burns chunk $N$, tripling flash throughput.
- **Nordic CoreSight CTRL-AP Unprotect**: Adds support for CoreSight Access Port #1 (`APSEL = 1`), enabling one-click mass-erase (`ERASEALL`) to recover locked nRF52/nRF53 devices protected by `APPROTECT`.

## Capabilities

### New Capabilities
- `segger-rtt-telemetry`: Background SRAM discovery of the `"SEGGER RTT"` control block, lock-free ring buffer draining over SWD, and WebSocket/terminal streaming.
- `network-gdb-server`: GDB Remote Serial Protocol (RSP) TCP server on port 3333 mapping target halt, step, registers, and memory directly to the Cortex-M SWD engine.
- `coresight-ctrlap-rescue`: Direct CoreSight Access Port 1 (`APSEL = 1`) addressing for automated mass-erase recovery of protected Nordic microcontrollers.

### Modified Capabilities
- `cmsis-dap`: Adds CMSIS-DAP v2 WinUSB Bulk endpoints and capability negotiation (`DAP_TransferBlock`, `DAP_SWJ_Sequence`, high-speed packet buffering).
- `generic-cortexm-algo-runner`: Adds double-buffered concurrent ping-pong flashing in target SRAM to overlap transport transfer with flash programming.

## Impact

- **Firmware (`firmware-rs`)**:
  - `src/usb.rs`: Adds CMSIS-DAP v2 WinUSB interface descriptors and bulk endpoints.
  - `src/programmer/dap.rs`: Upgrades DAP command processor with v2 block transfers and bulk dispatch.
  - `src/programmer/rtt.rs`: New module for background RTT discovery and ring buffer draining.
  - `src/programmer/gdb_server.rs`: New module implementing GDB Remote Serial Protocol over TCP.
  - `src/programmer/algo/mod.rs`: Double-buffered page execution pipeline.
  - `src/programmer/target/cortex_m.rs`: Multi-AP selection (`APSEL`) for CoreSight CTRL-AP access.
  - `src/rest.rs` & `src/web.rs`: Endpoints for RTT control and Nordic rescue (`/api/v1/programmer/rescue/nrf`).
- **Web UI (`web-ui/`)**:
  - Web Desktop Terminal (`/apps/terminal`) connects to RTT stream.
  - Target flasher shows live RTT output and Nordic unlock controls.
