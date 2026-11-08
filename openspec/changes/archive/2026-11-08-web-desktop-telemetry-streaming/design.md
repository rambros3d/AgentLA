## Context

AgentLA runs an Embassy async `#![no_std]` runtime on the Raspberry Pi RP2350 (ARM Cortex-M33, 150 MHz, 520 KiB SRAM). With True A/B dual-slot OTA firmware updates, LittleFS partition isolation, and target programmer engines established, the bench instruments currently rely on snapshot buffers (64 KiB), host CLI tools (`screen`/`picocom`), and manual API polling.

This design outlines the technical implementation for:
1. An **In-Browser Web Serial Terminal** (`/apps/terminal`) over native WebSockets.
2. **Continuous Multi-MB Logic Analyzer Streaming** with real-time Core 1 RLE compression.
3. An **OpenBench Logic Sniffer (OLS / Sump) TCP Server** on port 5555 for native PulseView integration.
4. A **Live Telemetry HUD** with real-time target rail power ($V_{\text{target}}$ / $I_{\text{target}}$ via TI INA180), MCU junction temperature, pin matrix visualization, and active short-circuit trip protection.

## Goals / Non-Goals

**Goals:**
- Provide zero-install browser serial communication with target MCUs at up to 921.6k baud.
- Stream continuous multi-second logic analyzer captures exceeding RP2350 SRAM limits.
- Allow PulseView to connect directly via `tcp/10.10.10.10/5555` with zero host drivers.
- Stream real-time 10 Hz target power and pin status to a floating desktop HUD.
- Protect target hardware with immediate High-Z pin trip on overcurrent events.

**Non-Goals:**
- Replacing specialized PC USB logic analyzers capable of gigabit-per-second uncompressed throughput.
- Full VT100 graphical terminal emulation beyond standard ANSI color and line editing.
- Modifying LittleFS filesystem layouts or firmware bootloader arbitration contracts.

## Architecture & System Flow

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                              BROWSER: WEB DESKTOP OS                                   │
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                        │
│   ┌───────────────────────────┐ ┌──────────────────────────┐ ┌──────────────────────┐  │
│   │ Floating Telemetry HUD    │ │ In-Browser Serial Term   │ │ Waveform Canvas      │  │
│   │ (10 Hz SSE Stream)        │ │ (/apps/terminal + WS)    │ │ (WASM Bit-Delta RLE) │  │
│   └─────────────┬─────────────┘ └────────────┬─────────────┘ └──────────┬───────────┘  │
│                 │                            │                          │              │
│                 │ GET /telemetry/stream      │ ws://10.10.10.10/ws      │ GET /stream  │
└─────────────────┼────────────────────────────┼──────────────────────────┼──────────────┘
                  │                            │                          │
┌─────────────────┼────────────────────────────┼──────────────────────────┼──────────────┐
│                 ▼                            ▼                          ▼              │
│   ┌────────────────────────────────────────────────────────────────────────────────┐   │
│   │                     CORE 0: EMBASSY ASYNC RUNTIME & PICOSERVE                  │   │
│   │                                                                                │   │
│   │   • picoserve HTTP/SSE engine + WebSocket upgrade (`picoserve/ws`)             │   │
│   │   • Background OLS/Sump TCP server listening on :5555                          │   │
│   │   • Pin Arbiter validation and overcurrent trip coordinator                    │   │
│   │   • Pulls compressed blocks from inter-core SPSC ring buffer                   │   │
│   └───────────────────────────────────┬────────────────────────────────────────────┘   │
│                                       │ 32 KiB Lock-Free SPSC Ring Buffer              │
│                                       ▼                                                │
│   ┌────────────────────────────────────────────────────────────────────────────────┐   │
│   │                     CORE 1: REAL-TIME DSP STREAMING ENGINE                     │   │
│   │                                                                                │   │
│   │   • Consumes 4 KiB ping-pong DMA buffers directly from PIO FIFO                │   │
│   │   • Adaptive RLE: Bit-Delta RLE for Web UI; Sump RLE for Sigrok                │   │
│   │   • Packets framed with sync word 0xAA55, seq_num, and overflow flag           │   │
│   └───────────────────────────────────┬────────────────────────────────────────────┘   │
│                                       │                                                │
│                                       ▼                                                │
│   ┌────────────────────────────────────────────────────────────────────────────────┐   │
│   │                        RP2350 HARDWARE PERIPHERALS                             │   │
│   │   [PIO Logic Analyzer DMA]  [UART1 Fixed Pins 8/9]  [PIO UART Flexible Pool]   │   │
│   │   [TI INA180 Current Amp]   [ADC2 Vtarget Sense]    [ADC4 Internal Temp Diode] │   │
│   └────────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                        │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

## Decisions & Trade-offs

### 1. In-Browser Serial Terminal Transport: Native WebSockets
- **Decision**: Activate the `ws` feature in `picoserve` to stream terminal bytes over a single full-duplex WebSocket connection.
- **Alternatives Considered**: Server-Sent Events (SSE) for RX + REST POST for TX.
- **Rationale**: SSE + POST uses 2 TCP sockets and incurs ~180 bytes of HTTP header overhead per keystroke, causing noticeable typing lag and connection pool pressure. WebSockets provide a true full-duplex pipe with 2-byte framing overhead and sub-millisecond character response.

### 2. Hybrid UART Driver (Fixed Hardware + Flexible PIO)
- **Decision**: Use hardware `UART1` on fixed pins GPIO 8/9; automatically switch to PIO state-machine UART when the user routes serial through flexible pool pins (e.g. GPIO 14/15).
- **Rationale**: Hardware UART provides zero CPU overhead and hardware FIFOs for default bench usage, while PIO provides arbitrary pin routing across all 30 GPIOs without hardwiring.

### 3. Dual-Core Allocation for Continuous Streaming
- **Decision**: Dedicate RP2350 Core 1 to hard-real-time dual ping-pong DMA reading and RLE compression, outputting to a lock-free SPSC queue for Core 0's TCP stack.
- **Rationale**: At 20–50 MSa/s, sample buffers fill in milliseconds. Running compression on Core 0 would induce jitter in Embassy async network processing and drop TCP packets. Dedicating Core 1 guarantees zero interrupt latency and steady streaming.

### 4. Adaptive Compression Engine
- **Decision**: Core 1 dynamically selects **Bit-Delta + Zero-Run RLE** when serving the browser Web UI (decompressed via WebAssembly/JS in the browser) and **Standard SUMP RLE** when serving PulseView on port 5555.
- **Rationale**: Bit-Delta prevents the 2x data expansion that standard RLE suffers when a fast clock line toggles. Standard SUMP RLE ensures native PulseView compatibility without requiring custom client plugins.

### 5. OLS/Sump Protocol Server on Port 5555
- **Decision**: Implement a background Embassy TCP listener on port 5555 supporting SUMP 1-byte control and 5-byte parameter commands, reporting up to 100 MHz sample rates and 16 channels.
- **Rationale**: PulseView already has a mature, built-in OpenBench Logic Sniffer TCP driver (`tcp/10.10.10.10/5555`). This provides immediate compatibility with existing bench software without distributing custom drivers.

### 6. Live Telemetry HUD with Hardware Overcurrent Trip
- **Decision**: Stream instrument telemetry at 10 Hz over SSE (`/api/v1/system/telemetry/stream`), combining TI INA180 current measurements, RP2350 internal ADC temperature, $V_{\text{SYS}}$, and the 30-pin ownership matrix into a floating glassmorphic HUD.
- **Safety**: If $I_{\text{target}}$ exceeds the programmed threshold, the Pin Arbiter immediately trips target interface pins to High-Z and sets an overcurrent alarm to protect both AgentLA and target hardware.

## Risks / Trade-offs

- **[Risk] High sample rates causing network buffer overruns**: If the network connection cannot sustain the data rate (e.g. noisy bus defeating RLE).
  - *Mitigation*: The SPSC ring buffer drops oldest frames, increments an overrun counter, and sets an explicit overflow bit in the frame header so the host waveform renderer marks the gap clearly.
- **[Risk] Picoserve `ws` feature size impact**: Adding WebSocket framing and SHA-1 handshake increases binary size by ~6–9 KiB.
  - *Mitigation*: Each firmware slot has 1.2 MB of allocated flash and our release binary is ~422 KiB, leaving >700 KiB of flash headroom.
- **[Risk] LittleFS corruption during live app seeding**: Seeding `/apps/terminal` could fail if flash is interrupted.
  - *Mitigation*: Apps are stored in the user filesystem partition above `0x00270000`, completely isolated from bootloader and firmware slots. A fallback embedded terminal HTML page is retained in ROM.
