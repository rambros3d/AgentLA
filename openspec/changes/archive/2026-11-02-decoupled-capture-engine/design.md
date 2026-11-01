# Design: Decoupled High-Speed Capture Engine

## Context
The RP2350's hardware USB PHY operates at USB 1.1 Full Speed (12 Mbps), providing an effective bulk transfer bandwidth of ~1.1 MB/s. However, multi-channel logic analysis and protocol decoding frequently demand sample rates of 20 MS/s to 150 MS/s. Transmitting raw sample data at 100 MS/s for 16 channels requires 200 MB/s, which depletes on-chip memory buffers in milliseconds. In practice, digital bus traffic consists primarily of static idle periods punctuated by edge transitions. By restructuring data representation and acquisition models, we decouple acquisition timing from USB delivery rates.

## Goals / Non-Goals

### Goals
- **Mode A (PIO RLE Streaming)**: Enable indefinite streaming of low/mid-speed buses (I2C, UART, CAN-FD, 1-Wire, slow SPI) over USB 1.1 with zero CPU overhead using PIO microcode encoding.
- **Mode B (Dual-Core Compression)**: Harness Core 1 with Cortex-M33 DSP/SIMD instructions to perform fast byte-delta and streaming compression on raw DMA ping-pong buffers at up to 100 MB/s.
- **Mode C (Circular Scope Acquisition)**: Provide an oscilloscope-style acquisition model using a 64 KB circular pre-trigger ring and 384 KB post-trigger linear buffer with single-cycle PIO trigger qualification.
- Provide runtime switching via CLI and WebUI.

### Non-Goals
- Adding secondary hardware bridge chips (e.g. CH32V305) or non-standard external cables (HDMI HSTX).
- Lossy compression that sacrifices sub-microsecond timing accuracy.

## Decisions

### 1. Packet Wire Format for PIO RLE (Mode A)
- **Format**: Fixed 32-bit packet: `[31:16] Run Duration (ticks)` | `[15:0] Pin State Bitmap`.
- **Alternative considered**: Variable-length quantity (VLQ) encoding. Rejected for PIO microcode because fixed 32-bit words map directly to 32-bit PIO `push` and 32-bit DMA transfers with zero instruction overhead.
- **Saturation / Keepalive**: If no transition occurs within 65,535 ticks, the state machine emits a packet with duration `0xFFFF` and identical pin state, ensuring real-time host keepalive.

### 2. Core 1 Pipeline and Memory Coordination (Mode B)
- **Buffer Layout**: Two 64 KB SRAM ping-pong arenas for DMA capture.
- **Inter-Core Coordination**: Lockless SIO FIFO flags (`multicore_fifo_push_blocking`) between Core 0 and Core 1. Core 1 consumes a completed 64 KB arena, computes word differences using `EOR` instructions, and outputs compressed blocks into a shared USB transmit ring buffer consumed by Core 0's TinyUSB worker.

### 3. Oscilloscope Pre/Post-Trigger Buffer Partitioning (Mode C)
- **Buffer Distribution in 520 KB SRAM**:
  - `Pre-Trigger Ring`: 64 KB circular DMA ring (`channel_config_set_ring(&cfg, true, 16)`).
  - `Post-Trigger Linear Buffer`: 384 KB contiguous SRAM.
  - Remaining ~72 KB reserved for system stack, lwIP TCP/IP buffers, and USB endpoint pools.
- **Trigger Qualifier**: A dedicated PIO state machine evaluates edge, pattern, or duration conditions. Upon a trigger match, an internal hardware IRQ activates the post-trigger DMA channel. Once filled, capture stops and the unified 448 KB buffer is transferred to the host PC in ~390 ms.

## Risks / Trade-offs

- **[Risk] High-frequency glitch storm in Mode A causing RLE packet expansion (2x data rate)**  
  → *Mitigation*: PIO DMA overflow detector checks FIFO backlog. If incoming transitions exceed 250,000 edges/s, the driver flags buffer backpressure and prompts the user to switch to Mode C (Triggered Scope Mode).

- **[Risk] Core 1 starvation if Core 0 USB endpoint stalls**  
  → *Mitigation*: Bounded ring buffer with drop counters; Core 0 signals flow-control pause to Core 1 if the USB host halts polling.

- **[Risk] SRAM contention between DMA, Core 0, and Core 1**  
  → *Mitigation*: Assign DMA ping-pong buffers and Core 1 scratch memory to striped SRAM banks (SRAM0–SRAM7) to eliminate bus arbitration contention.
