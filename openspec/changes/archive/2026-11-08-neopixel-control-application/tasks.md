## 1. Firmware LED WebSocket Streaming Endpoint

- [x] 1.1 Implement `LedWebSocket` handler in `firmware-rs/src/rest.rs` with dynamic pin claiming and releasing via `pin_owner`
- [x] 1.2 Implement multi-protocol binary frame dispatch (WS2812/SK6812 1-wire via PIO/SIO, and APA102/WS2801 2-wire via SPI)
- [x] 1.3 Register route `/api/v1/led/ws` in `firmware-rs/src/web.rs`

## 2. Neopixel Control Web Application Architecture

- [x] 2.1 Scaffold `apps/neopixel-control/app.json` with metadata, tags, capabilities, and `icon.svg`
- [x] 2.2 Build `apps/neopixel-control/index.html` structure with DaisyUI v5 bento layout, linking `/daisyui.css` and `/style.css`
- [x] 2.3 Implement Protocol Selector (WS2812, SK6812 RGBW, WS2811, WS2813, WS2815, APA102, WS2801, LPD8806) and color order mapping

## 3. Visualizer, Color Picker & 60 FPS Procedural Shaders

- [x] 3.1 Implement interactive 1D strip/ring canvas with manual brush, range selection, and fill tools
- [x] 3.2 Implement 2D matrix view with Serpentine (zigzag), Row-Major, and Column-Major coordinate mapping
- [x] 3.3 Implement HSV Color Wheel, preset swatches, and dedicated White Channel slider for SK6812 RGBW
- [x] 3.4 Implement 60 FPS procedural shaders (Rainbow, Cylon, Fire, Meteor, Pulse, Plasma) streaming frames over WebSocket
- [x] 3.5 Implement real-time estimated current (mA) and power warning badge with USB threshold color coding

## 4. App Store Integration, Packaging & Tests

- [x] 4.1 Register `neopixel-control` in `apps/catalog.json` and generate `apps/packages/neopixel-control.tar`
- [x] 4.2 Run `python3 tools/pack_webui.py` to verify LittleFS system partition headroom (>35% free)
- [x] 4.3 Add automated unit tests in `test/test_neopixel_app.py` verifying app manifest, WebSocket routing, and catalog discovery
- [x] 4.4 Run `cargo check --manifest-path firmware-rs/Cargo.toml` and pytest suite to verify zero regressions
