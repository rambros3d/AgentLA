# ols-sump-pulseview-server

## Purpose
Specifies the native OpenBench Logic Sniffer (OLS / Sump) TCP server listening on port 5555, providing direct PulseView and Sigrok desktop waveform capture integration up to 100 MHz without custom drivers.

## Requirements

### Requirement: OpenBench Logic Sniffer TCP Server on Port 5555
The firmware SHALL run a background Embassy async TCP listener on port 5555 implementing the standard OpenBench Logic Sniffer (OLS / Sump) protocol for zero-install PulseView and Sigrok desktop integration.

#### Scenario: PulseView Device Probe and Identification
- **WHEN** PulseView connects to `tcp/10.10.10.10/5555` and transmits 1-byte command `0x02` (Query ID)
- **THEN** server immediately responds with 4-byte ASCII signature `1ALS`.

#### Scenario: Hardware Metadata Exchange
- **WHEN** client sends 1-byte command `0x04` (Query Metadata)
- **THEN** server responds with a TLV packet detailing device name ("AgentLA RP2350 Logic Bench"), firmware version ("v1.0.0-rust"), max sample rate (100 MHz), memory size (1 MSamples), probe count (16), and protocol flags (RLE supported).

### Requirement: OLS Hardware Trigger and Divider Configuration
The OLS server SHALL parse 5-byte parameter commands to configure PIO clock divider, sample window depth, channel masks, and 4-stage trigger conditions.

#### Scenario: Sample Rate Configuration
- **WHEN** client sends command `0x80` with a 24-bit divider value
- **THEN** the firmware calculates $f = \frac{100\,\text{MHz}}{\text{divider} + 1}$ and loads the value into the PIO clock prescaler.

#### Scenario: Trigger Mask and Value Arming
- **WHEN** client sends trigger configuration commands `0xC0`–`0xC7` followed by `0x01` (Run)
- **THEN** the firmware configures PIO trigger logic matching the specified pin masks and transitions to the armed state.

### Requirement: Dual Mode Capture Operation
The OLS server SHALL support both fixed-depth Snapshot mode (returning a finite sample block) and Continuous Streaming mode over the TCP socket.

#### Scenario: Finite Snapshot Capture
- **WHEN** client configures read count via `0x81` and issues `0x01` Run
- **THEN** the server captures the exact pre-trigger and post-trigger sample count and transmits the buffer to the client before returning to IDLE.

#### Scenario: Real-Time Sump RLE Wire Framing
- **WHEN** client enables the RLE flag via command `0x82`
- **THEN** sample data is framed in SUMP RLE format where bit 7 denotes repeat count (`1xxxxxxx`) or raw sample value (`0xxxxxxx`).
