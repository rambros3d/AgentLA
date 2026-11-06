## 1. SAR ADC 4-Channel Oscilloscope Engine

- [x] 1.1 Create `firmware-rs/src/instruments/mod.rs` and `firmware-rs/src/instruments/oscilloscope.rs` with RP2350 SAR ADC registers, channel round-robin sequencing (ADC 0–3 on GPIO 26–29), and circular DMA buffer transfers.
- [x] 1.2 Implement waveform edge/voltage threshold trigger detection and metric calculations ($V_{pp}, V_{rms}$, frequency, duty cycle).
- [x] 1.3 Implement dynamic pin ownership lifecycle for GPIO 26–29 (`CAP_ADC`), guaranteeing high-Z digital release when scope is idle.
- [x] 1.4 Expose REST snapshot endpoint `/api/v1/oscilloscope/data` and Server-Sent Events (SSE) stream `/api/v1/oscilloscope/stream` in `firmware-rs/src/web.rs`.

## 2. Digital Signal & Clock Generator

- [x] 2.1 Implement `firmware-rs/src/instruments/generator.rs` with RP2350 hardware PWM slice allocation across GPIO 10–29.
- [x] 2.2 Implement frequency calculation (1 Hz to 75 MHz), duty cycle resolution (0% to 100%), and finite burst pulse counter.
- [x] 2.3 Expose REST endpoints `/api/v1/generator/start`, `/api/v1/generator/stop`, and `/api/v1/generator/status` in `firmware-rs/src/web.rs`.

## 3. WS2812 Timing Margin Analyzer & Pattern Generator

- [x] 3.1 Implement `firmware-rs/src/instruments/ws2812.rs` with PIO 800 kHz transmitter to drive standard RGB/RGBW color patterns.
- [x] 3.2 Implement PIO edge timestamping receiver to measure $T_{0H}, T_{1H}, T_{0L}, T_{1L}$, and Reset latch timings with tolerance check.
- [x] 3.3 Expose REST endpoints `/api/v1/led/pattern` and `/api/v1/led/analyze` in `firmware-rs/src/web.rs`.

## 4. Hardware Fault & Glitch Injector

- [x] 4.1 Implement `firmware-rs/src/instruments/glitch.rs` with cycle-accurate PIO engine (150 MHz, $6.67\,\text{ns}$ step resolution).
- [x] 4.2 Support external trigger edge arming, programmable delay (10 ns to 10 ms), and pulse-width generation down to 13.3 ns.
- [x] 4.3 Expose REST endpoints `/api/v1/glitch/arm`, `/api/v1/glitch/disarm`, and `/api/v1/glitch/status` in `firmware-rs/src/web.rs`.

## 5. Scope Tab UI & Stimulus Deck Integration

- [x] 5.1 Connect Scope Tab (`#view-scope` in `firmware/html/index.html`) waveform canvas to live SSE data stream `/api/v1/oscilloscope/stream`.
- [x] 5.2 Wire the bottom collapsible stimulus drawer controls to `/api/v1/generator/*`, `/api/v1/led/*`, and `/api/v1/glitch/*`.

## 6. Build, Flashing & Hardware Verification

- [x] 6.1 Recompile `firmware-rs` release build and generate UF2 binary.
- [x] 6.2 Flash physical RP2350 probe and verify live oscilloscope SSE stream and PWM clock output via pin loopback.
- [x] 6.3 Remove completed Section 1 items from `FUTURE.md` and commit all artifacts cleanly to git.
