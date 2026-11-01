## 1. Phase 1: Dynamic Pin Routing for Bus Adapters

- [x] 1.1 Add interactive pin selection modals/drawers for I2C (GP10/11, GP14/15, GP18/19, GP26/27) and SPI in `firmware/html/app.js` and `index.html`
- [x] 1.2 Update adapter lifecycle in `app.js` to release existing claim handles before acquiring new pin sets
- [x] 1.3 Add pin validation preventing claims on fixed programmer block (GP0–9)

## 2. Phase 2: Arbitrary Dynamic Capture Span & Digital Scope

- [x] 2.1 Add base pin (GP10–22) and channel count (1–8) controls to the Logic Analyzer header
- [x] 2.2 Wire arbitrary span parameters into `CAPTURECLAIM` and PIO0 channel mapping
- [x] 2.3 Implement digital scope timing analysis in `app.js` calculating frequency, period, and duty cycle from captured edge transitions

## 3. Phase 3: Dedicated Signal Generator Instrument

- [x] 3.1 Implement PWM/Clock signal output command handlers (`OUTCLAIM`, `OUTSET`, `OUTRELEASE`) in `firmware/src/main.c`
- [x] 3.2 Add Signal Generator view template `#template-generator` with frequency, duty cycle, and pulse sliders in `firmware/html/index.html`
- [x] 3.3 Add Signal Generator tile to the Dashboard and navigation rail item
- [x] 3.4 Wire generator control surface and live output status in `firmware/html/app.js`

## 4. Phase 4: Dedicated Analog Scope Instrument

- [x] 4.1 Add Analog Scope view template `#template-scope` with 4-channel voltage canvas and trigger controls in `firmware/html/index.html`
- [x] 4.2 Add Analog Scope tile to the Dashboard and navigation rail item
- [x] 4.3 Implement ADC continuous stream sampling in `firmware/src/main.c`
- [x] 4.4 Wire dual-personality lifecycle in `firmware/html/app.js` (claiming GP26–29 on launch, releasing on close)

## 5. Phase 5: Automated Internal Bus Sniffer Presets

- [x] 5.1 Add "Sniff I2C" and "Sniff SPI" buttons to the top-bar quick presets
- [x] 5.2 Implement automated dual-claim logic in `app.js` (Driver for Adapter + Observer for Logic Analyzer on identical pins)
- [x] 5.3 Configure split view automatic mounting (Adapter in Slot 1, Logic in Slot 2)

## 6. Public Portal Sync & Firmware Verification

- [x] 6.1 Mirror updated templates, styling, and simulator bridge into `web/` and `web/demo/`
- [x] 6.2 Compile firmware (`make -C build-work AgentLA`) and verify zero errors and minimal binary size
- [x] 6.3 Run unit tests (`pytest test/test_sigrok_export.py`)
