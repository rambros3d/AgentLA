## ADDED Requirements

### Requirement: Real-Time Core 1 Compression Engine
The firmware SHALL allocate Core 1 exclusively to consuming raw sample blocks from DMA ping-pong buffers, executing Cortex-M33 DSP-accelerated bit-difference and LZ4 compression, and pushing compressed chunks to a lockless ring buffer for Core 0 USB transmission.

#### Scenario: Real-time compression under dense logic activity
- **WHEN** raw sample data enters a completed 64 KB DMA ping-pong buffer on Core 1
- **THEN** Core 1 processes the 64 KB block using ARMv8-M `EOR` instructions within 1.0 ms and deposits the compressed chunk into the shared USB endpoint queue

### Requirement: Adaptive Flow Control and Compression Backpressure
The compression engine SHALL monitor the output queue fill level and flag backpressure if the uncompressed signal stream cannot be reduced below the USB transfer rate.

#### Scenario: Backpressure notification on dense high-speed bursts
- **WHEN** the compressed output exceeds the USB endpoint drain rate for three consecutive buffers
- **THEN** Core 1 signals an overflow warning to Core 0, increments a dropped-frame metric, and reports the condition to the host interface
