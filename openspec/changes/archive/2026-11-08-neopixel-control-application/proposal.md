## Why

Engineers, makers, and embedded developers frequently work with addressable LEDs (WS2812B, SK6812, WS2811, WS2813, WS2815, APA102, WS2801) for UI indicators, robotics, sensor feedback, and interactive lighting. However, testing and animating LED strips or 2D matrix panels typically requires writing, compiling, and flashing custom microcontroller code (e.g. FastLED or Adafruit NeoPixel sketches) over USB, leading to slow iterative feedback loops and power supply brownout risks. 

AgentLA has an RP2350 microcontroller with 12 PIO state machines, cycle-accurate timing drivers, and pin arbitration. Delivering a dedicated, user-installable **Neopixel Control** application (`apps/neopixel-control`) backed by a dedicated 60 FPS binary WebSocket streaming endpoint (`/api/v1/led/ws`) provides instantaneous, hands-free hardware control, color manipulation, procedural shaders, multi-protocol selection, and current/power telemetry directly from any web browser.

## What Changes

- Create the **Neopixel Control** user application in `apps/neopixel-control` (`index.html`, `app.json`, `icon.svg`), leveraging DaisyUI v5 components and the layout utility layer.
- Support both **1-wire clockless** protocols (WS2812, WS2812B, SK6812 RGBW with dedicated white channel, WS2811, WS2813, WS2815, APA106) and **2-wire clocked SPI** protocols (APA102 / DotStar, SK9822, WS2801, LPD8806) with selectable color orderings (GRB, RGB, BGR, RGBW, GRBW).
- Support both **1D linear strips / rings** (1 to 512 pixels) and **2D matrix panels** (e.g., 8×8, 16×16, 8×32) with configurable wiring topologies (Serpentine/Zigzag, Row-Major, Column-Major).
- Implement interactive painting tools (HSV color wheel, preset swatches, white channel slider, brush, fill bucket) and high-performance 60 FPS client-side procedural shaders (Rainbow, Cylon, Fire, Meteor, Pulse, Plasma).
- Implement a real-time power and current advisory badge calculating estimated milliamps and warning on USB overcurrent thresholds (>500mA, >1500mA).
- Implement a dedicated firmware WebSocket server at `/api/v1/led/ws` in `firmware-rs/src/rest.rs` and `web.rs` supporting JSON config handshakes, dynamic GPIO pin claiming via `pin_owner`, and raw binary frame streaming at up to 60 FPS.
- Register `neopixel-control` in `apps/catalog.json` and generate `apps/packages/neopixel-control.tar` for the built-in App Store.

## Capabilities

### New Capabilities
- `neopixel-control-application`: Interactive addressable LED workbench application for testing, visual editing, and animating 1D strips and 2D matrix panels across multiple FastLED-compatible protocols.
- `led-websocket-streaming`: Dedicated low-latency WebSocket endpoint `/api/v1/led/ws` for dynamic pin configuration, multi-protocol driver dispatch (PIO and SPI), and high-throughput binary frame streaming.

### Modified Capabilities
- `community-app-store`: The official app catalog and packaging pipeline SHALL include the `neopixel-control` application package alongside existing community applications.

## Impact

- **Applications**: New app added at `apps/neopixel-control/`, packaged into `apps/packages/neopixel-control.tar`, and cataloged in `apps/catalog.json`.
- **Firmware**: Route `/api/v1/led/ws` added in `firmware-rs/src/web.rs`, with WebSocket handler in `firmware-rs/src/rest.rs` dispatching to `bridge/ws2812.rs` and `bridge/spi.rs` with dynamic pin arbitration.
- **WebUI Storage**: WebUI partition continues to comfortably house system assets with >35% headroom.
- **Tests**: Automated integration tests added to verify app manifest, WebSocket protocol handling, and catalog validation.
