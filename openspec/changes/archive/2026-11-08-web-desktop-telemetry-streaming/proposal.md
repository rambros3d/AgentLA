## Why

AgentLA has proven hardware capabilities for target programming, logic analysis, and dual-slot True A/B OTA, but user interaction is currently split across disjointed snapshot APIs, curl commands, and separate desktop terminal emulators. 

This change elevates AgentLA into a self-contained, real-time bench instrument by introducing:
1. An **In-Browser Serial Terminal** (`/apps/terminal`) eliminating the need for host serial tools (`picocom`/`screen`).
2. **Continuous Multi-MB Logic Analyzer Streaming** with real-time Core 1 RLE compression, breaking the 64 KiB snapshot barrier.
3. An **OpenBench Logic Sniffer (OLS / Sump) TCP Server** on port 5555 enabling zero-install, 1-click desktop PulseView captures up to 100 MHz.
4. A **Live Telemetry HUD** providing real-time target power ($V_{\text{target}}$, $I_{\text{target}}$ via TI INA180), silicon health, pin ownership matrix, and hardware overcurrent trip safety.

## What Changes

- **In-Browser Web Serial Terminal (`/apps/terminal`)**:
  - Full-duplex WebSocket connection via `picoserve/ws` (`ws://10.10.10.10/api/v1/terminal/ws`) with sub-millisecond typing latency.
  - Hybrid UART driver (hardware `UART1` on default fixed GPIO 8/9; dynamic PIO UART fallback for flexible pool pins).
  - 2 KiB circular backlog in SRAM to preserve target boot messages before the browser window opens.
  - Packaged as a LittleFS app in `/apps/terminal/` bundled into `webui.tar`.
  - Frontend controls: Baud selector (up to 921.6k), pin picker, 1-click `nRESET` pulse button, and log export.

- **Continuous Multi-MB Streaming & Core 1 RLE Engine**:
  - Dual-core task division: Core 0 runs network & web; Core 1 runs a dedicated hard-real-time dual ping-pong DMA consumer.
  - Dynamic channel widths: 4, 8, or 16 channels selectable at runtime.
  - Adaptive compression: Bit-Delta + Zero-Run RLE for Web UI canvas; Standard Sump RLE for Sigrok.
  - Congestion handling: Overrun drops oldest data with an explicit overflow flag in the stream packet header.

- **OpenBench Logic Sniffer (OLS / Sump) TCP Protocol Server**:
  - Dedicated background Embassy TCP listener on port 5555.
  - Standard SUMP 1-byte and 5-byte command dispatch (Reset, Arm, Query ID `1ALS`, Metadata, Divider, Triggers).
  - Reports up to 100 MHz max sample rate (1 sample/PIO clock for 8 channels).
  - Supports both Snapshot mode (fixed buffer) and Continuous streaming mode.
  - Out-of-the-box PulseView / Sigrok desktop compatibility without host drivers.

- **Live Telemetry HUD & Power Protection**:
  - Hybrid delivery: 10 Hz SSE push stream (`/api/v1/system/telemetry/stream`) with REST polling fallback (`/api/v1/system/telemetry`).
  - Target power metrics ($V_{\text{target}}$ and $I_{\text{target}}$ via TI INA180 shunt amplifier).
  - Silicon diagnostics: RP2350 internal junction temperature, $V_{\text{SYS}}$ input voltage, SRAM / LittleFS fill rates.
  - Real-time 30-pin color-coded Pin Ownership Matrix.
  - Active bench protection: Configurable target overcurrent trip alarm (trips pins to High-Z on short circuits).
  - Floating, draggable glassmorphic HUD overlay across all Web Desktop windows.

## Capabilities

### New Capabilities
- `in-browser-serial-terminal`: In-browser xterm.js serial terminal app with full-duplex WebSocket streaming, hybrid UART driver, 2 KiB SRAM backlog, and target reset pulse control.
- `continuous-dma-streaming-rle`: Continuous multi-megabyte logic analyzer capture pipeline with Core 1 real-time DSP, adaptive Bit-Delta + Zero-Run and Sump RLE compression, dynamic 4/8/16 channel widths, and ring buffer overflow protection.
- `ols-sump-pulseview-server`: Native OpenBench Logic Sniffer (OLS / Sump) TCP server listening on port 5555 for direct, zero-install PulseView / Sigrok desktop waveform captures up to 100 MHz.
- `live-telemetry-hud`: Real-time floating glassmorphic telemetry HUD (10 Hz SSE stream) displaying target rail voltage/current ($V_{\text{target}}$, $I_{\text{target}}$ via INA180), MCU core temperature, $V_{\text{SYS}}$, memory gauges, active pin ownership matrix, and programmable overcurrent trip protection.

### Modified Capabilities
<!-- No requirement changes to existing specs; builds alongside existing web desktop and analyzer specs -->

## Impact

- **Firmware (`firmware-rs`)**:
  - Activates `ws` feature on `picoserve` (`picoserve = { version = "0.20.1", features = ["defmt", "embassy", "ws"] }`).
  - Implements WebSocket terminal service at `/api/v1/terminal/ws`.
  - Implements OLS/Sump TCP server background task on port 5555.
  - Spawns Core 1 real-time RLE streaming engine for continuous captures.
  - Implements telemetry SSE stream at `/api/v1/system/telemetry/stream`.
- **Crates**:
  - `agentla-analyzer`: Exposes adaptive RLE and Sump RLE compression blocks.
  - `agentla-bridges`: Hybrid UART driver integration with async ring buffers.
  - `agentla-bsp`: INA180 telemetry conversion and ADC temperature calculation.
- **Frontend (`webui/`)**:
  - Adds `/apps/terminal/` app with xterm.js bundle.
  - Adds floating glassmorphic Telemetry HUD component to `app.js` and `apps.html`.
  - Updates `tools/pack_webui.py` to bundle new terminal app into `webui.tar`.
