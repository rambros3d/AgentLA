## Context

AgentLA provides embedded protocol adapters, signal analyzers, and hardware instrumentation directly from the RP2350 dual-core microcontroller. Currently, the addressable LED subsystem in `firmware-rs/src/bridge/ws2812.rs` and `firmware-rs/src/instruments/ws2812.rs` has basic driver capability and an endpoint for timing analysis (`/api/v1/led/analyze`), but lacks a dedicated user-facing application, dynamic pin selection across arbitrary GPIOs, multi-protocol framing support (for FastLED-style chipsets like APA102, SK6812 RGBW, WS2811, WS2813), and high-throughput real-time frame streaming.

By pairing a dedicated WebSocket streaming endpoint (`/api/v1/led/ws`) with an interactive, DaisyUI v5-styled application in `apps/neopixel-control`, users can configure and drive LED strips and 2D matrix displays with zero compile/flash steps.

## Goals / Non-Goals

**Goals:**
- Provide a clean, standalone user application in `apps/neopixel-control` conforming to DaisyUI v5 design guidelines and AgentLA bento conventions.
- Support both 1-wire clockless protocols (WS2812, WS2812B, SK6812 RGBW, WS2811, WS2813, WS2815, APA106) and 2-wire clocked SPI protocols (APA102/DotStar, SK9822, WS2801, LPD8806) with configurable color ordering.
- Support interactive visual editing and real-time animation of 1D strips/rings and 2D matrix grids (with Serpentine/Zigzag and Progressive layout options).
- Establish a dedicated WebSocket endpoint (`/api/v1/led/ws`) for sub-millisecond, low-latency binary frame streaming at up to 60 FPS.
- Integrate dynamic pin claiming via `pin_owner` so the user can assign any available GPIO (0–29) for Data and optional Clock without hardcoded pin lockups.
- Display a real-time power and current advisory badge calculating estimated current consumption ($mA$) with color-coded safety warnings.

**Non-Goals:**
- Autonomous microcontroller-side compiled C++/Arduino shader uploading. Animations run client-side in the browser and stream frames to the device over WebSocket.
- Driving more than 1024 LEDs simultaneously in a single string (limited by RP2350 RAM and refresh rate timing).

## Decisions

### Decision 1: Dedicated WebSocket Endpoint (`/api/v1/led/ws`) over HTTP REST
* **Choice**: Implement `/api/v1/led/ws` modeled after `CanWebSocket` and `TerminalWebSocket`.
* **Rationale**: HTTP POST introduces HTTP header overhead, connection establishment, and request/response serialization latency (~10–25ms per frame), which restricts frame rates to 15–30 FPS with noticeable stutter. A persistent WebSocket with binary payloads allows streaming 60 FPS RGB buffers (e.g. 180 bytes for 60 LEDs) in sub-millisecond dispatch cycles.
* **Alternatives Considered**: HTTP POST with hex string body. Kept as fallback, but WebSocket is primary for live animations.

### Decision 2: Client-Side Shaders & Procedural Animation Engine
* **Choice**: Execute procedural shaders (Rainbow, Fire, Cylon, Meteor, Plasma) in JavaScript using `requestAnimationFrame`.
* **Rationale**: Gives users endless creative flexibility, rich color palettes, sound reactivity, text banner rendering, and instant parameter tuning without requiring firmware updates or complex byte-code interpreters on the RP2350.
* **Alternatives Considered**: Built-in firmware shader loops. Rejected because firmware loops cannot be customized by user web apps without firmware re-flashing.

### Decision 3: Dynamic Pin Ownership Contract
* **Choice**: The WebSocket configuration handshake specifies `data_pin` and optional `clk_pin`. The firmware acquires ownership via `pin_owner::claim(Driver::Ws2812, &[data_pin], ...)` and releases them when the WebSocket disconnects or when Safe State is triggered.
* **Rationale**: Upholds the AgentLA architecture contract that no driver may touch GPIOs without holding an active lease from `pin_owner`.

## Risks / Trade-offs

- **[Risk] High Current & USB Brownout**: Driving 60+ LEDs at full white can draw over 3 Amps, easily tripping host USB port protections.
  → **Mitigation**: The app calculates real-time estimated mA consumption based on RGBW values and prominently displays an amber/red current warning badge when exceeding 500mA / 1500mA.
- **[Risk] WebSocket Backpressure / Network Congestion**: If the browser sends frames faster than the physical LED bitstream can transmit, packets could pile up.
  → **Mitigation**: The client throttles transmissions to match the protocol frame rate ($T_{frame} = N_{pixels} \times 30\mu s + 50\mu s$) and only sends the next frame upon receipt of the previous frame's 1-byte ACK.
