## Why

Embedded hardware engineers debugging microcontroller targets frequently face mixed-signal anomalies—such as power rail droops during radio TX bursts, sensor analog voltage ramps during digital bus transactions, LED timing violations, and clock-fault lockups. Today, developers must alternate between separate external bench instruments or switch between disconnected views.

By leveraging the RP2350's hardware peripherals (SAR ADC on GPIO 26–29, 8 PWM slices across GPIO 10–29, and nanosecond-resolution PIO state machines), AgentLA can provide a complete, interactive **Analog & Mixed-Signal Instrument Suite** unified directly within the primary **Scope Tab**.

---

## What Changes

1. **4-Channel 500 kS/s Analog Oscilloscope (`analog-oscilloscope`)**:
   - Continuous SAR ADC sampling on GPIO 26–29 (ADC 0–3) at up to 500 kS/s single-channel or 125 kS/s 4-channel round-robin via circular DMA.
   - Real-time voltage edge and threshold trigger detection.
   - Hardware metrics computation ($V_{pp}, V_{rms}$, frequency, duty cycle).
   - High-throughput Server-Sent Events (SSE) data stream (`/api/v1/oscilloscope/stream`) and snapshot endpoint (`/api/v1/oscilloscope/data`).
   - Dynamic pin ownership lifecycle: returns GP26–GP29 to flexible digital pool when oscilloscope is stopped or closed.

2. **Digital Signal & Arbitrary Clock Generator (`digital-signal-generator`)**:
   - Hardware PWM slice engine outputting clock references (1 Hz to 75 MHz) and variable PWM (0% to 100% duty cycle) on any flexible pin (GPIO 10–29).
   - Burst pulse generator (outputting exactly $N$ clock pulses) to stimulate clocked ICs or recover MCUs with halted internal oscillators.
   - REST control endpoints (`/api/v1/generator/start`, `/api/v1/generator/stop`, `/api/v1/generator/status`).

3. **Addressable RGB LED Tester & Timing Margin Analyzer (`ws2812-timing-analyzer`)**:
   - PIO-based pattern generator driving standard GRB/RGBW sequences (rainbow sweeps, solid colors).
   - PIO-based line timing analyzer measuring $T_{0H}, T_{1H}, T_{0L}, T_{1L}$, and Reset latch delays to verify tolerance margins and detect bus degradation.
   - REST endpoints (`/api/v1/led/pattern`, `/api/v1/led/analyze`).

4. **Hardware Fault & Glitch Injector (`hardware-glitch-injector`)**:
   - Cycle-accurate PIO engine operating at 150 MHz ($6.67\,\text{ns}$ step resolution).
   - Synchronized trigger delay (10 ns to 10 ms) and nanosecond pulse width control (down to 13.3 ns / 2 clock cycles) on flexible GPIO.
   - Brownout resilience evaluation and hardware security diagnostic endpoints (`/api/v1/glitch/arm`, `/api/v1/glitch/disarm`, `/api/v1/glitch/status`).

5. **Scope Tab Frontend Integration**:
   - Embeds 4-channel analog graticule alongside digital logic lanes within the primary **Scope Tab** (`#view-active-bench` / `#view-scope`).
   - Integrates the **Collapsible Stimulus Drawer** in the bottom bench deck to control Signal Generator, WS2812, and Glitch Injection without crowding the waveform canvas.

---

## Capabilities

### New Capabilities
- `ws2812-timing-analyzer`: Dedicated PIO pattern transmitter and nanosecond-resolution timing analyzer for WS2812B/SK6812 LED lines.
- `hardware-glitch-injector`: Cycle-accurate PIO clock and voltage glitch injection engine for hardware security and fault resilience testing.

### Modified Capabilities
- `analog-oscilloscope`: Expanded to support multi-channel 125 kS/s round-robin sampling, live SSE streaming at `/api/v1/oscilloscope/stream`, and DaisyUI MSO canvas rendering.
- `digital-signal-generator`: Expanded to support burst pulse count generation and unified control via `/api/v1/generator/*` REST endpoints.

---

## Impact

- **Firmware (`firmware-rs`)**:
  - Adds `firmware-rs/src/instruments/` module (`oscilloscope.rs`, `generator.rs`, `ws2812.rs`, `glitch.rs`).
  - Connects ADC DMA, PWM slices, and PIO state machines.
  - Exposes REST and SSE routes in `firmware-rs/src/web.rs`.
- **Frontend (`firmware/html/`)**:
  - Scope Tab (`#view-scope` in `index.html`) renders superimposed analog waveforms, trigger cursor, and measurement stats.
  - Bottom collapsible drawer provides interactive controls for Signal Generator, WS2812 patterns, and Glitch arming.
- **Pin & Resource Safety**:
  - Strict dynamic pin arbitration via `pin_owner` ensuring ADC and PWM allocations never collide with active digital bus bridges or debug probe lines.
