## 1. Dependencies & Base Transport Activation

- [x] 1.1 Enable `ws` feature on `picoserve` in `firmware-rs/Cargo.toml`
- [x] 1.2 Verify clean compilation on `thumbv8m.main-none-eabihf` with WebSocket support enabled

## 2. In-Browser Serial Terminal (/apps/terminal)

- [x] 2.1 Implement hybrid UART driver with hardware `UART1` (fixed GPIO 8/9) and async PIO UART fallback in `firmware-rs/src/bridge/uart.rs`
- [x] 2.2 Implement 2 KiB circular SRAM backlog buffer for incoming target serial bytes
- [x] 2.3 Implement WebSocket handler `GET /api/v1/terminal/ws` with query arg parsing in `firmware-rs/src/web.rs`
- [x] 2.4 Build self-contained `/apps/terminal/index.html` with xterm.js, baud switcher, pin selector, local echo, and target reset pulse
- [x] 2.5 Seed `/apps/terminal` in `firmware-rs/src/apps.rs` and update `tools/pack_webui.py`

## 3. Continuous Multi-MB Streaming & Core 1 RLE DSP

- [x] 3.1 Implement dual 4 KiB ping-pong DMA channels in `firmware-rs/src/analyzer.rs` supporting 4, 8, and 16 channels
- [x] 3.2 Implement Core 1 hard-real-time consumer with adaptive Bit-Delta + Zero-Run and SUMP RLE compression
- [x] 3.3 Implement 32 KiB inter-core lock-free SPSC ring buffer with frame sequence numbering and overflow flagging
- [x] 3.4 Expose continuous binary streaming endpoint at `GET /api/v1/analyzer/stream`

## 4. OpenBench Logic Sniffer (OLS / Sump) TCP Server (:5555)

- [x] 4.1 Implement background Embassy TCP listener task on port 5555 in `firmware-rs/src/ols.rs`
- [x] 4.2 Implement 1-byte command dispatch: Reset (`0x00`), Run (`0x01`), Query ID `1ALS` (`0x02`), and Metadata TLV (`0x04`)
- [x] 4.3 Implement 5-byte parameter parser: Set Divider (`0x80`), Read Count (`0x81`), Flags (`0x82`), and Trigger Masks/Values (`0xC0`–`0xC7`)
- [x] 4.4 Connect OLS server to Core 1 capture pipeline with Sump RLE wire streaming

## 5. Live Telemetry HUD & Power Safety

- [x] 5.1 Implement ADC sampling for INA180 target current ($I_{\text{target}}$), rail voltage ($V_{\text{target}}$), internal junction temp, and $V_{\text{SYS}}$
- [x] 5.2 Implement programmable hardware overcurrent trip monitor with immediate Pin Arbiter High-Z release
- [x] 5.3 Implement 10 Hz SSE push stream at `GET /api/v1/system/telemetry/stream` and REST polling at `GET /api/v1/system/telemetry`
- [x] 5.4 Build floating collapsible glassmorphic HUD overlay in `webui/app.js` and `webui/apps.html` with 30-pin matrix visualization

## 6. System Verification & OTA Packaging

- [x] 6.1 Unit test OLS command parser, TLV metadata, and Sump RLE wire framing on `x86_64`
- [x] 6.2 Live hardware verification of `/apps/terminal` WebSocket connection and target serial echo
- [x] 6.3 Live hardware verification of PulseView connecting via `tcp/10.10.10.10/5555`
- [x] 6.4 Package updated firmware via True A/B OTA, flash over Ethernet, and verify LittleFS app seeding
