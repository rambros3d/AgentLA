# Design: Analog & Mixed-Signal Instruments

## Context

AgentLA's RP2350 microcontroller possesses rich hardware capabilities: a 5-channel 500 kS/s 12-bit SAR ADC, 8 dual-channel PWM slices, and two programmable I/O (PIO) blocks running at 150 MHz. While the primary observation interface ("Scope" tab) was designed to show mixed-signal waveforms, the Rust firmware (`firmware-rs`) currently lacks the underlying instrument engines for ADC DMA acquisition, PWM signal output, WS2812 timing verification, and cycle-accurate glitching.

---

## Goals / Non-Goals

### Goals
- Implement a 4-channel analog oscilloscope engine using RP2350 SAR ADC (GPIO 26–29) and circular DMA buffers (up to 500 kS/s single-channel, 125 kS/s round-robin 4-channel).
- Implement hardware and software edge/threshold triggering with real-time waveform metrics ($V_{pp}, V_{rms}$, frequency, duty cycle).
- Stream live oscilloscope frames to the web frontend using Server-Sent Events (SSE) at `/api/v1/oscilloscope/stream` and snapshot REST `/api/v1/oscilloscope/data`.
- Implement a flexible digital signal & clock generator (1 Hz to 75 MHz, 0–100% PWM, burst pulse counter) using RP2350 hardware PWM slices across GPIO 10–29.
- Implement a WS2812B/SK6812 PIO pattern transmitter and timing margin analyzer.
- Implement a cycle-accurate nanosecond PIO fault and clock glitch injector.
- Integrate the frontend controls into the primary **Scope Tab** and its bottom collapsible stimulus drawer.
- Preserve strict pin safety via `pin_owner` dynamic claim arbitration.

### Non-Goals
- Megasample (>1 MS/s) sampling (bounded by the RP2350 internal SAR ADC architecture).
- Arbitrary analog waveform DAC output (RP2350 lacks an internal DAC; PWM filtering is used for analog output).

---

## Architecture & Technical Decisions

### 1. Module Structure (`firmware-rs/src/instruments/`)

```
firmware-rs/src/
├── instruments/
│   ├── mod.rs                  # Module coordinator & pin lifecycle helpers
│   ├── oscilloscope.rs         # RP2350 SAR ADC DMA acquisition & trigger engine
│   ├── generator.rs            # Hardware PWM slice manager (1 Hz – 75 MHz & bursts)
│   ├── ws2812.rs               # PIO LED pattern transmitter & timing receiver
│   └── glitch.rs               # High-precision PIO fault & clock glitch engine
```

### 2. SAR ADC & DMA Engine (`oscilloscope.rs`)
- **Channels**: ADC 0 (GP26), ADC 1 (GP27), ADC 2 (GP28), ADC 3 (GP29).
- **ADC Configuration**:
  - ADC clock: 48 MHz. Clock divider programmed for 500 kS/s rate.
  - Round-robin mask (`RROBIN`): allows sampling 1, 2, 3, or all 4 channels sequentially into the ADC FIFO.
  - DMA Channel: Transfers 12-bit samples from `ADC_FIFO` into a circular RAM buffer (e.g., 4096 samples).
- **Trigger Detection**:
  - Evaluates threshold crossing on selected trigger channel (rising, falling, or level).
  - Pre-trigger and post-trigger windows preserved using circular buffer offset calculation.
- **Waveform Telemetry**:
  - Computes min, max, peak-to-peak voltage ($V_{pp} = V_{\max} - V_{\min}$), RMS voltage ($V_{rms} = \sqrt{\frac{1}{N}\sum V_i^2}$), and estimated frequency from zero/threshold crossings.

### 3. Data Streaming & Web Transport
- **SSE Stream (`/api/v1/oscilloscope/stream`)**:
  - Emits JSON/hex-encoded chunk events on trigger completion:
    ```
    event: frame
    data: {"ch_mask":15,"sample_rate":125000,"vpp":[3.3,1.6,0,0],"samples":"..."}
    ```
- **REST Snapshot (`/api/v1/oscilloscope/data`)**:
  - Returns one-shot buffer of raw samples and calibrated millivolt values.

### 4. PWM Clock & Signal Generator (`generator.rs`)
- Maps user GPIO pin ($10 \le \text{pin} \le 29$) to PWM slice ($\lfloor\text{pin}/2\rfloor \bmod 8$) and channel ($\text{pin} \bmod 2$).
- Computes optimal integer divider, fractional divider (0–15), and `TOP` counter wrap value for target frequency.
- **Burst Mode**: Employs PWM interrupt or PIO counter to shut off output after exactly $N$ pulses.

### 5. WS2812 Timing Analyzer (`ws2812.rs`)
- **Transmitter PIO**: 800 kHz standard WS2812 bit timing ($T_{0H} \approx 350\,\text{ns}$, $T_{1H} \approx 700\,\text{ns}$).
- **Analyzer PIO**: Measures high and low pulse durations by counting clock cycles between edges.
- Validates against tolerance windows:
  - $T_{0H}$: $350\,\text{ns} \pm 150\,\text{ns}$
  - $T_{1H}$: $700\,\text{ns} \pm 150\,\text{ns}$
  - Reset Latch: $\ge 50\,\mu\text{s}$

### 6. Cycle-Accurate Fault Glitcher (`glitch.rs`)
- PIO program:
  1. Wait for trigger pin edge (GPIO IN).
  2. Decrement delay counter (down to $6.67\,\text{ns}$ resolution at 150 MHz).
  3. Drive glitch pin low/high for specified duration.
  4. Return pin to default state.

---

## Risks / Trade-offs & Mitigations

| Risk | Mitigation |
|---|---|
| **ADC Pin Contention**: User tries to capture logic analyzer digital channels on GP26–29 while ADC is active. | `pin_owner` arbiter strictly rejects `CAP_ADC` if pins are held by digital drivers, and releases pins back to digital pool on scope close. |
| **CPU Starvation during ADC Sampling**: High-rate ADC streaming consumes core cycles. | ADC uses DMA to transfer data autonomously into memory without CPU polling. |
| **SSE Network Backpressure**: Web client slow to consume high frame-rate SSE events. | Firmware skips frame emission if client socket buffer is full (`WouldBlock`), preserving smooth real-time display without lagging. |
