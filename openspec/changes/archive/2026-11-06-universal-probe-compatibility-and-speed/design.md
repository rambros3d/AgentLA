## Context

AgentLA already provides a multi-protocol hardware platform with SWD, UART, I2C, SPI, CAN-FD, and a dual LittleFS partition architecture running on Raspberry Pi RP2350. However, standard embedded development workflows still center around mainstream IDEs and command-line tools such as VS Code (`cortex-debug`), Keil MDK, CLion, `probe-rs`, OpenOCD, and native `gdb`. 

To achieve 100% interoperability with the global embedded ecosystem while adopting the performance and telemetry innovations of commercial debug probes (SEGGER J-Link, NXP MCU-Link, and Black Magic Probe), AgentLA needs:
1. Native USB CMSIS-DAP v2 Bulk endpoints for driverless zero-latency operation with host tools.
2. Background Segger RTT (Real-Time Transfer) memory scanning to stream target logs without consuming UART pins.
3. An on-probe GDB Remote Serial Protocol (RSP) TCP server on port 3333 over CDC-NCM Ethernet (`10.10.10.10`), eliminating the need for host-side OpenOCD daemons.
4. Concurrent double-buffered turbo flashing to triple firmware programming throughput.
5. CoreSight CTRL-AP mass-erase support to unlock protected Nordic nRF52/nRF53 microcontrollers.

---

## Goals / Non-Goals

### Goals
- **Universal Toolchain Compatibility**: Provide plug-and-play detection and debugging in `probe-rs`, VS Code, Keil MDK, and OpenOCD via USB CMSIS-DAP v2 Bulk.
- **Zero-Pin Telemetry**: Stream target logs non-intrusively from target SRAM into the AgentLA Web Desktop terminal via background SWD RTT polling without halting the CPU.
- **Direct Remote GDB**: Allow any host machine to run `arm-none-eabi-gdb -ex "target remote 10.10.10.10:3333"` to debug target hardware directly over network/USB.
- **Turbo Flashing**: Double-buffer target SRAM during CMSIS-Pack algorithm execution to overlap SWD transport transfer with on-chip flash programming.
- **Nordic Silicon Rescue**: Provide one-click recovery of protected nRF52/nRF53 devices via CoreSight Access Port #1 (`CTRL-AP`).

### Non-Goals
- Proprietary Segger J-Link USB vendor emulation (CMSIS-DAP v2 and GDB RSP provide superior open interoperability without reverse-engineering encrypted protocols).
- Full instruction trace over ETM/ETB in this change (focusing on RTT memory tracing and SWD CoreSight).

---

## Decisions

### 1. CMSIS-DAP v2 WinUSB Bulk Interface
* **Decision**: Add a dedicated USB interface with two Bulk endpoints (Bulk IN and Bulk OUT) exposing the CMSIS-DAP v2 protocol descriptor and Microsoft OS 2.0 WinUSB descriptors.
* **Command Dispatch**: The bulk endpoint listener receives 64-byte or 512-byte packets, dispatches standard DAP commands (`DAP_Connect`, `DAP_Disconnect`, `DAP_Transfer`, `DAP_TransferBlock`, `DAP_SWJ_Clock`, `DAP_SWJ_Sequence`), and responds directly from the RP2350 PIO SWD state machine.
* **Rationale**: CMSIS-DAP v2 removes the 1 ms HID polling latency limit of v1, achieving multi-megabyte transfer speeds while being natively recognized by Windows 10/11, Linux, and macOS without custom drivers.

### 2. Segger RTT Background Engine
* **Decision**: Implement an asynchronous RTT monitor in `firmware-rs/src/programmer/rtt.rs`:
  * Searches the first 64 KB of target SRAM (`0x20000000`–`0x20010000`) for the 16-byte signature `"SEGGER RTT\0\0\0\0\0\0"`.
  * Extracts the `SEGGER_RTT_CB` struct containing buffer descriptors:
    - `aUp[0].pBuffer`: pointer to text ring buffer
    - `aUp[0].SizeOfBuffer`: ring buffer capacity
    - `aUp[0].WrOff`: target CPU write index
    - `aUp[0].RdOff`: probe read index
  * Drains unread bytes over non-intrusive SWD memory reads without halting target core, and broadcasts chunks over WebSocket to Web Desktop Terminal (`/apps/terminal`).
* **Rationale**: RTT achieves sub-microsecond logging latency on target MCUs with zero GPIO pin overhead.

### 3. On-Probe GDB Server (TCP Port 3333)
* **Decision**: Run a TCP server listening on `10.10.10.10:3333` using `embassy-net`:
  * Parses GDB Remote Serial Protocol (RSP) packets with packet framing (`$<data>#<checksum>`).
  * Maps core GDB commands to existing `CortexM` methods:
    - `$?`: query halt reason $\rightarrow$ `DHCSR` halt status
    - `$g` / `$G`: read/write general registers R0–R15, xPSR
    - `$m` / `$M`: read/write memory blocks
    - `$s` / `$c`: single step / continue
    - `$Z0` / `$z0`: hardware breakpoint insert / remove
* **Rationale**: Follows the Black Magic Probe paradigm, completely removing the requirement for OpenOCD or local servers on the developer's PC.

### 4. Concurrent Double-Buffered Turbo Flashing
* **Decision**: In `FlashAlgorithmRunner`, split available target SRAM into two alternating buffers (`Buffer 0` and `Buffer 1`):
  1. Write data chunk 0 to `Buffer 0` and execute `ProgramPage(Buffer 0)`.
  2. While target CPU is programming flash for chunk 0, stream data chunk 1 into `Buffer 1` over SWD simultaneously.
  3. Wait for `BKPT 0` on chunk 0, then immediately trigger `ProgramPage(Buffer 1)` and fill `Buffer 0`.
* **Rationale**: Overlaps USB/SWD communication latency with flash cell programming time, increasing flashing speed by ~3×.

### 5. Nordic CoreSight CTRL-AP Support
* **Decision**: Extend `CortexM` and `SwdTransport` with CoreSight AP selection (`APSEL` in `SELECT` register `0x08`):
  * Select Access Port 1 (`APSEL = 1`).
  * Write `0x01` to register `0x004` (`ERASEALL`).
  * Poll register `0x008` (`ERASEALLSTATUS`) until erase completes.
  * Reset target to restore AHB-AP #0 access.
* **Rationale**: Bypasses the silicon `APPROTECT` lockout without requiring external vendor dongles.

---

## Risks / Trade-offs

| Risk | Mitigation |
| :--- | :--- |
| **Simultaneous CMSIS-DAP and WebUI access** | `PinArbiter` enforces exclusive claim; if CMSIS-DAP is active, WebUI flasher returns `409 Conflict` until released. |
| **Target memory access while target is running (RTT)** | ARM CoreSight AHB-AP allows transparent memory reads/writes via TAR/DRW while CPU is running without halting. |
| **GDB RSP TCP connection drop during halt** | Server detects disconnect, releases debug halt, or resumes target safely to avoid orphan target hangs. |
| **Target SRAM too small for double buffering** | Fallback to single-buffer mode if target SRAM window in `AlgoBlobHeader` is < 2 KB. |
