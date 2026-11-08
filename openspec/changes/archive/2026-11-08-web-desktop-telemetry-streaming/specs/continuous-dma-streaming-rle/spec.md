## ADDED Requirements

### Requirement: Dual-Core Continuous Logic Analyzer Pipeline
The firmware SHALL execute continuous logic analyzer sampling across both RP2350 cores: Core 0 SHALL manage network streaming and client socket connections, while Core 1 SHALL run a dedicated hard-real-time dual ping-pong DMA consumer loop without operating system preemption.

#### Scenario: Continuous Ping-Pong DMA Processing
- **WHEN** continuous logic capture is armed
- **THEN** hardware DMA channels alternately fill two 4 KiB buffers from the PIO sample FIFO while Core 1 processes the inactive buffer, maintaining uninterrupted capture across multi-second durations.

### Requirement: Dynamic Channel Width Selection
The streaming engine SHALL support configurable channel widths of 4, 8, or 16 channels selectable at capture arming.

#### Scenario: 8-Channel Standard Capture
- **WHEN** client requests 8 channels
- **THEN** PIO packs 8 digital bits into 1 byte per sample and feeds DMA with 8-bit granularity.

#### Scenario: 16-Channel Wide Bus Capture
- **WHEN** client requests 16 channels
- **THEN** PIO packs 16 digital bits into 2 bytes per sample and feeds DMA with 16-bit word granularity.

### Requirement: Adaptive Real-Time RLE DSP
Core 1 SHALL support adaptive compression modes: it SHALL encode with Bit-Delta + Zero-Run RLE for Web UI canvas streams, and SHALL encode with standard SUMP RLE when serving desktop Sigrok/Pulseview clients.

#### Scenario: Bit-Delta Web UI Streaming
- **WHEN** a Web UI client connects to `/api/v1/analyzer/stream`
- **THEN** Core 1 applies XOR delta transformation and zero-run compression, suppressing high-frequency single-pin clock toggles from causing data expansion.

#### Scenario: Standard Sump RLE Desktop Streaming
- **WHEN** desktop PulseView connects to the OLS TCP server on port 5555 with RLE flags enabled
- **THEN** Core 1 encodes consecutive identical sample words into standard SUMP `(count, sample)` byte pairs.

### Requirement: Congestion Handling and Overflow Flagging
When host network consumption stalls or TCP throughput falls behind sample generation, the SPSC ring buffer SHALL drop the oldest unconsumed packets and assert an explicit overflow flag in the subsequent frame header.

#### Scenario: Network Congestion Recovery
- **WHEN** the 32 KiB inter-core SPSC buffer reaches capacity
- **THEN** the streaming engine discards the oldest block, increments an overflow counter, and sets the overflow bit in the next transmitted sync header so the receiver marks the timeline discontinuity.
