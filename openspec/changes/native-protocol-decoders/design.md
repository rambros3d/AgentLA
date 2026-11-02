# Design: Native Protocol Decoders (Phases 1–5)

## Context
AgentLA currently provides embedded decoders for I2C, SPI, UART, and PWM. However, embedded systems, automotive engineering, and industrial robotics demand deeper protocol inspection. Siling Probe has dedicated transceivers for CAN-FD and RS-485, and 16-channel logic pins capable of capturing 1-Wire, I3C, and addressable LED streams. This design establishes a high-performance, modular C decoding subsystem running entirely within the RP2350 firmware without external dependencies.

## Goals / Non-Goals

### Goals
- Implement high-accuracy, zero-heap C protocol decoders for:
  - Phase 1: CAN 2.0B / CAN-FD, Modbus RTU, DMX-512
  - Phase 2: Universal Addressable LEDs (1-wire PWM WS2812/SK6812/UCS1903 + 2-wire clocked APA102/DotStar/WS2801)
  - Phase 3: 1-Wire Dallas (DS18B20 temperature auto-decode), LIN Bus, MIPI I3C SDR
  - Phase 4: ARM SWD Sniffer, ARM SWO ITM Trace Printf, SPI Flash Command Interpreter
  - Phase 5: WebUI Canvas Visual Overlays with CSS RGB swatches and SSE live stream
- Maintain strictly deterministic memory footprint ($\le 32\,\text{KB}$ RAM for event tables).
- Expose decoders via both CLI (`CAPTURE mode=DECODED protocol=...`) and interactive WebUI.

### Non-Goals
- Full bus simulation or active master emulation (these decoders operate as passive sniffers/analyzers on captured or streaming logic data).
- Heavy optical video decoding (e.g. DVI/HDMI).

## Decisions

### 1. Compact Union-Based Event Model (`DECODER_EVENT_V2`)
- **Structure**: Replace fixed 64-byte text strings with a 24-byte compact struct containing a union of protocol-specific decoded structures.
- **Rationale**: Storing 1,024 decoded events requires only 24 KB of RAM, compared to >70 KB with text strings. Formatting to human-readable strings occurs lazily at CLI print time or JSON serialization.

### 2. CAN 2.0B & CAN-FD Bitstream Engine
- **Bit Recovery**: Operates with a sample point phase accumulator ($75\%-80\%$ of nominal and data-phase bit cells).
- **Bit De-stuffing**: Maintains a 5-bit consecutive run counter. A 6th matching bit is flagged as a `Stuff Error`. Stuff bits are stripped prior to CRC computation.
- **CRC Validation**: Computes CRC-15 (polynomial `0x4599` for Classic CAN), CRC-17 (`0x1685B` for CAN-FD $\le 16$ bytes), and CRC-21 (`0x102899` for CAN-FD $> 16$ bytes).

### 3. Modbus RTU Delimitation via $3.5\times t_{\text{char}}$ Silent Interval
- **Framing**: Character times are computed from the baud rate ($11 \times \frac{1}{\text{baud}}$ for 8-E-1 / 8-N-2). Any inter-byte idle gap exceeding $3.5 \times t_{\text{char}}$ marks an end-of-frame packet delimiter.
- **Integrity**: Standard Modbus CRC-16 polynomial `0xA001` (LSB first) validates message integrity before decoding function codes and register boundaries.

### 4. Unified Addressable LED Engine
- **Single-Wire Engine**: Measures high-pulse duration on every bit cell ($1.25\,\mu\text{s}$ nominal at 800 kHz). Classifies bits as `0` ($0.2-0.5\,\mu\text{s}$) or `1` ($0.65-0.95\,\mu\text{s}$). Flags timing violations and glitch pulses exceeding $1.4\,\mu\text{s}$.
- **Two-Wire Clocked Engine**: Uses SPI clock edges to sample data synchronously. Decodes 32-bit words per LED, extracting 5-bit Global Brightness ($0-31$) and 24-bit BGR/RGB values.

### 5. 1-Wire Dallas & Automatic DS18B20 Parsing
- **Handshake Verification**: Measures Reset LOW duration ($480-960\,\mu\text{s}$) and Presence pulse ($60-240\,\mu\text{s}$).
- **Application Extraction**: When command `0xBE` (Read Scratchpad) is detected, extracts temperature bytes 0 and 1, verifies 8-bit DOW CRC ($x^8 + x^5 + x^4 + 1$), and computes exact Celsius temperature:
  $$\text{Temp} = \frac{\text{raw\_value}}{16.0}\,{^\circ\text{C}}$$

### 6. ARM SWD Sniffer
- **Synchronization**: Detects JTAG-to-SWD switching sequences (`0x79E7` or `0xE79E`) and line reset sequences (50+ high bits).
- **Packet Extraction**: Decodes Start bit, APnDP, RnW, A[2:3] address bits, Parity bit, 3-bit ACK (`OK=1`, `WAIT=2`, `FAULT=4`), and 32-bit WDATA/RDATA with data parity verification.

### 7. WebUI Visual Canvas Overlays & CSS Swatches
- In [`firmware/html/app.js`](file:///run/media/sun/halo/AgentLA/firmware/html/app.js), the canvas renderer displays colored annotation blocks aligned with sample timestamps:
  - CAN: Green bubbles for Arbitration IDs, cyan for payload bytes, yellow for CRC.
  - Modbus: Purple bubbles for Function Codes (`FC: 03 Read Holding`), cyan for register numbers.
  - Addressable LEDs: Visual color swatches rendering the exact hex color of every decoded pixel (e.g. `<span style="background:#FF5500"></span> #FF5500`).

## Risks / Trade-offs

- **[Risk] Sample Rate Aliasing on High-Speed CAN-FD (e.g. 5 Mbps data phase)**  
  → *Mitigation*: Ensure minimum $8\times$ oversampling ($40\,\text{MS/s}$) for 5 Mbps CAN-FD. The decoder checks sample rate and alerts if oversampling is insufficient.

- **[Risk] High Event Count Overflowing the 1,024 Event Arena on Long Traces**  
  → *Mitigation*: The event arena acts as a rolling buffer, keeping the latest 1,024 events while maintaining an aggregate frame/error counter.

- **[Risk] Microsecond Timing Jitter on 1-Wire Reset Pulses**  
  → *Mitigation*: Implement standard Dallas timing bounds with $\pm 20\%$ tolerance windows.
