## 1. Mode A: In-Hardware PIO RLE Streaming

- [x] 1.1 Implement `pio_rle_capture` program in `firmware/pio/capture.pio` with 32-bit `[RunLength:16 | PinState:16]` packing and saturation keepalive.
- [x] 1.2 Update `firmware/src/analyzer/capture.c` to support RLE capture mode, initializing DMA channel for 32-bit word streaming.
- [x] 1.3 Add host-side RLE packet unpacker in Python test suite and WebUI Sigrok/VCD exporter.

## 2. Mode B: Dual-Core Compression Pipeline

- [x] 2.1 Implement Cortex-M33 DSP bit-delta encoder in `firmware/src/rle_util.c` utilizing single-cycle `EOR` instructions.
- [x] 2.2 Configure Core 1 worker task in `firmware/src/main.c` / `analyzer/capture.c` to consume 64 KB DMA ping-pong buffers via SIO FIFO.
- [x] 2.3 Connect compressed block stream to Core 0 TinyUSB bulk endpoint queue with backpressure flow control.

## 3. Mode C: Circular Pre/Post-Trigger Oscilloscope Acquisition

- [x] 3.1 Implement circular 64 KB pre-trigger DMA ring configuration (`channel_config_set_ring`) and 384 KB post-trigger linear buffer in `capture.c`.
- [x] 3.2 Implement fast PIO trigger qualifier in `capture.pio` supporting pattern, edge, and pulse-width conditions that fire hardware IRQ to start post-trigger DMA.
- [x] 3.3 Add capture completion handler that freezes capture and initiates unified 448 KB buffer dump over USB 1.1.

## 4. Unified Control Surface, CLI & WebUI

- [x] 4.1 Implement `CAPTUREMODE <STREAM|TURBO|SCOPE>` command in `firmware/src/main.c` and `webui_command_bridge.c`.
- [x] 4.2 Add UI capture mode selector pill in `firmware/html/app.js` and `web/index.html` allowing dynamic switching between Stream, Turbo, and Scope modes.
- [x] 4.3 Add automated unit tests verifying RLE encoding, trigger qualification, and Sigrok export fidelity.
