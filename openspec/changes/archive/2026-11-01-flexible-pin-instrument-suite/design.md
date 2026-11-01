## Context

AgentLA implements dynamic pin ownership in `firmware/src/pin_owner.c`, dividing the RP2350's 30 pins into a fixed programmer block (`GPIO 0–9`) and a flexible dynamic pool (`GPIO 10–29`). While the backend supports dynamic claims, the user interface and high-level firmware dispatch currently use static pin defaults for adapters and capture banks.

This design covers the architecture for:
1. Dynamic pin pickers for bus adapters (I2C, SPI, UART).
2. Arbitrary dynamic capture span sizing for the Logic Analyzer & Digital Scope.
3. A dedicated Signal Generator (PWM/clock/pulse) instrument.
4. A dedicated Analog Scope (ADC) instrument on GP26–29.
5. 1-tap automated internal sniffer presets.

## Goals / Non-Goals

**Goals:**
- Allow users to select any valid pin combination in GPIO 10–29 for I2C, SPI, and UART.
- Allow users to select any starting pin (GP10–29) and channel count (up to 8 channels) for logic capture.
- Provide real-time digital scope timing measurements (frequency, duty cycle, pulse widths).
- Provide a dedicated Signal Generator instrument with frequency (1 Hz–50 MHz) and duty cycle sliders.
- Provide a dedicated Analog Scope instrument on GP26–29 with dual-role lifecycle.
- Provide 1-tap auto-sniff presets (Adapter driver + Logic observer).

**Non-Goals:**
- Allowing flexible role claims on GPIO 0–9 (strictly reserved for programmer hardware).
- Modifying analog inputs beyond GP26–29 (hardware ADC mux constraint).

## Decisions

### 1. Adapter Pin Routing
- In `app.js`, each adapter UI renders a "Pin Config" button.
- Opening the config displays valid pin pairs based on RP2350 `IO_BANK0` alternate functions and PIO fallback.
- Selecting a pair releases any existing adapter handle and executes `PINCLAIM pins=<csv> func=<id> observer=0`.

### 2. Logic Analyzer Arbitrary Span Sizing
- The Logic Analyzer header features a "Span" control:
  - Base Pin: Dropdown from GP10 to GP22.
  - Channel Count: 1 to 8 channels (up to maximum span 8).
- When "Run" is clicked, it calls `CAPTURECLAIM pins=<selected_pins> trigger=<trig_pin>` to configure PIO0 input base.

### 3. Dedicated Signal Generator Instrument
- Implemented as a top-level instrument view (`template-generator`) and rail item.
- Uses RP2350 hardware PWM slices (`pwm_set_wrap`, `pwm_set_chan_level`).
- Firmware command: `OUTCLAIM pin=<x> mode=PWM`, `OUTSET pin=<x> freq=<hz> duty=<0-100>`, `OUTRELEASE pin=<x>`.

### 4. Dedicated Analog Scope Instrument
- Implemented as a top-level instrument view (`template-scope`) and rail item.
- Claims GP26–29 with `PINCLAIM pins=26,27,28,29 func=ADC`.
- Samples ADC at up to 500 kS/s and plots voltage curves on a dedicated scope canvas.
- When navigated away or closed, automatically calls `PINRELEASE` to free GP26–29 back to the digital pool.

### 5. Automated Sniffer Preset
- The top-bar presets menu adds "Sniff I2C" and "Sniff SPI".
- When triggered:
  - Claims adapter driver on GP10,11 (I2C) or GP14,15,16,17 (SPI).
  - Claims capture bank on the identical pins with `observer=1`.
  - Sets split view mode with Adapter in Left Slot and Logic in Right Slot.

## Risks / Trade-offs

- **[Contention with Fixed Block]** User might attempt to drag capture spans into GP0–9.
  - *Mitigation*: The UI strictly bounds selectable base pins to $\ge 10$, and firmware `pin_owner.c` rejects any claim $< 10$.
- **[ADC Contention]** User might leave Analog Scope open while trying to use GP26–29 for digital pins.
  - *Mitigation*: The ownership registry returns an informative conflict alert, and switching instruments auto-quiesces the ADC.
