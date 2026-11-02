# Change: Decoupled High-Speed Capture Engine

## Why
The RP2350's integrated USB 1.1 Full-Speed PHY is physically limited to ~1.1 MB/s bulk throughput, while embedded logic buses (I2C, SPI, UART, CAN, JTAG) require 20–150 MS/s sampling resolution ($40 - 300\,\text{MB/s}$ raw data). Because digital buses are static for 95%–99.9% of sample ticks, transmitting uncompressed samples wastes bus bandwidth and overflows buffers in milliseconds. Decoupling the sampling rate from the physical wire rate via microcode RLE, multi-core compression, and oscilloscope-style circular acquisition enables continuous, lossless logic capture on BakuProbe and Pico 2.

## What Changes
- Add **Mode A: In-Hardware PIO RLE Streaming (`pio-rle-streaming`)**: Zero-CPU microcode encoder emitting 32-bit `[RunLength:16 | PinState:16]` packets directly into DMA, enabling indefinite continuous streaming of I2C, UART, CAN-FD, and 1-Wire without host memory limits.
- Add **Mode B: Dual-Core Compression Pipeline (`dual-core-compression-pipeline`)**: Assign Core 1 exclusively to real-time ARMv8-M DSP/SIMD delta and fast LZ4 compression of raw DMA blocks before dispatching compressed chunks to Core 0 TinyUSB bulk endpoints.
- Add **Mode C: Circular Oscilloscope Pre/Post-Trigger FIFO (`oscilloscope-trigger-fifo`)**: Dedicated 64 KB pre-trigger circular DMA ring + 384 KB post-trigger buffer in 520 KB SRAM with hardware PIO trigger qualification for single-shot 100–150 MS/s capture and 390 ms USB drain.
- Add CLI commands: `CAPTUREMODE <STREAM|TURBO|SCOPE>`, `RLETHRESHOLD <ticks>`, `TRIGGERCONFIG <mask,match,type,pre_pct>`.
- Expose streaming mode selection and trigger setup in the WebUI.

## Capabilities

### New Capabilities
- `pio-rle-streaming`: Zero-CPU hardware run-length encoding in PIO microcode with duration/state packet packing for infinite USB 1.1 streaming.
- `dual-core-compression-pipeline`: Core 1 real-time bit-delta and LZ4 compression engine feeding compressed blocks into USB bulk endpoints.
- `oscilloscope-trigger-fifo`: Oscilloscope-style circular pre-trigger ring and post-trigger acquisition with PIO hardware qualification and post-capture USB transfer.

### Modified Capabilities
- `arbitrary-capture-span`: Modified to support RLE-encoded streaming mode and circular pre/post-trigger ring sizing alongside raw linear captures.

## Impact
- `firmware/pio/capture.pio`: New microcode programs for RLE encoding and trigger qualifier.
- `firmware/src/analyzer/capture.c`: Updated DMA ping-pong orchestration and ring buffer management.
- `firmware/src/rle_util.c` & `firmware/src/rle_util.h`: Fast Cortex-M33 DSP bit-delta compression functions.
- `firmware/src/main.c`: CLI commands for capture mode selection and trigger configuration.
- `firmware/html/app.js`: WebUI support for selecting capture modes (Stream, Turbo, Scope) and configuring pre-trigger depth.
