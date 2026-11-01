## Why

While AgentLA's firmware implements dynamic pin ownership (`firmware/src/pin_owner.c`), flexible routing across GPIO 10–29 remains partially locked down in the user interface and firmware tool abstractions:
1. Adapters (I2C, SPI, UART) default to hardcoded pin assignments rather than letting users choose valid pin combinations in the flexible pool.
2. The Logic Analyzer currently assumes a base pin of GP20 rather than allowing the user to select an arbitrary start pin and channel width across GPIO 10–29.
3. Digital Signal Generation (PWM frequency/duty, clock outputs, and pulse trains) is not yet exposed as an interactive bench tool.
4. Analog Oscilloscope capability on GP26–29 lacks a dedicated live waveform display and dual-personality release mechanism.
5. The "internal bus sniffer" capability (simultaneous adapter driver + logic capture observer on the same physical pins) lacks a 1-tap automated setup.

Unifying these into a fully flexible instrument suite unlocks the full potential of the RP2350's PIO and hardware peripheral architecture without external jumper loops.

## What Changes

### Phase 1: Dynamic Pin Routing for Bus Adapters
- Introduce interactive pin-picker selectors in the I2C, SPI, and UART adapter interfaces.
- Users can choose any valid hardware or PIO-backed pin combination in GPIO 10–29 (e.g., I2C on GP10/11, GP14/15, GP18/19, or GP26/27).
- Commands dynamically pass handles via `PINCLAIM` and `PINRELEASE`.

### Phase 2: Arbitrary Dynamic Capture Span & Digital Scope
- Upgrade the Logic Analyzer interface with an arbitrary span selector: start pin (GP10–GP29) and channel width (1 to 8 channels).
- Implement Digital Scope timing measurements: real-time period, frequency, pulse width ($T_{\text{high}}$, $T_{\text{low}}$), and duty cycle calculations calculated from high-speed edge transitions.

### Phase 3: Dedicated Signal Generator Instrument (Digital Outputs)
- Add a new top-level bench instrument: **Signal Generator** on the dashboard and navigation rail.
- Support RP2350 hardware PWM and PIO clock generation:
  - Select any pin from GPIO 10–29.
  - Set frequency from 1 Hz up to 50 MHz and duty cycle from 0% to 100%.
  - Output burst pulse counts for hardware target triggering.
- Implement firmware commands `OUTCLAIM`, `OUTSET`, `OUTRELEASE`.

### Phase 4: Dedicated Analog Scope Instrument (GP26–29 Dual Mode)
- Add a new top-level bench instrument: **Analog Scope** on the dashboard and navigation rail.
- Samples RP2350 4-channel 500 kS/s SAR ADC on GP26–29 with live voltage waveform rendering ($0\text{V} - 3.3\text{V}$).
- Enforces dual-role lifecycle: releases analog pins back to the flexible digital pool when the instrument is closed or idle.

### Phase 5: 1-Tap Auto-Sniffer Split Presets
- Add automated split-view presets ("Sniff I2C", "Sniff SPI"):
  - Claims the bus master adapter as a driver on flexible pins (e.g., GP10/11).
  - Automatically claims the Logic Analyzer as an observer on the identical physical pins.
  - Mounts both tools side-by-side in Split View, allowing bus transactions to be captured internally without external jumper wiring.

## Capabilities

### New Capabilities
- `flexible-adapter-routing`: Dynamic pin assignment and validation for I2C, SPI, and UART across GPIO 10–29.
- `arbitrary-capture-span`: Dynamic base pin and width selection for Logic Analyzer capture banks.
- `digital-signal-generator`: PWM, clock, and pattern generation instrument on GPIO 10–29.
- `analog-oscilloscope`: 4-channel ADC waveform oscilloscope instrument for GPIO 26–29 with dual-role lifecycle.
- `internal-bus-sniffer`: Automated simultaneous driver + observer split-view orchestration.

### Modified Capabilities
<!-- None: Protocol commands retain backward compatibility with claim handles -->

## Impact

- **Firmware**: Added `cmd_out` (signal generator) in `firmware/src/main.c`, updated adapter pin configurations.
- **WebUI**: Added Signal Generator and Analog Scope templates and nav icons in `firmware/html/index.html`, added routing logic in `firmware/html/app.js`, and updated `style.css`.
- **Public Portal**: Synchronized to `web/` and `web/demo/`.
- **Flash Size**: Lightweight additions keep RP2350 binary well within flash limits.
