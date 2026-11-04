## ADDED Requirements

### Requirement: Universal Dual-Family Addressable LED Transmission
The HTTP server SHALL provide `POST /api/v1/bus/led/transmit` supporting both 1-wire PWM/NRZ (WS2812B, SK6812 RGB & 32-bit RGBW, WS2811 800k/400k, WS2813) and 2-wire clocked SPI (APA102/DotStar, SK9822, WS2801) for up to 256 pixels with configurable color order, global brightness scaling, and hardware reset gaps.

#### Scenario: Transmitting WS2812B GRB pixel stream
- **WHEN** client sends `POST /api/v1/bus/led/transmit` with `{ "pin": 10, "protocol": "1wire", "model": "ws2812b", "pixels": [{"r": 255, "g": 0, "b": 0}, {"r": 0, "g": 255, "b": 0}] }`
- **THEN** server configures `neopixel.pio` on GPIO 10, swizzles pixels to GRB order, streams 48 bits, holds reset line low for >280 µs, and returns HTTP 200 with `{ "status": "ok", "pixels_sent": 2, "protocol": "1wire" }`.

#### Scenario: Transmitting SK6812 32-bit RGBW pixel stream
- **WHEN** client sends `POST /api/v1/bus/led/transmit` with `{ "pin": 10, "protocol": "1wire", "model": "sk6812_rgbw", "pixels": [{"r": 0, "g": 0, "b": 0, "w": 255}] }`
- **THEN** server configures 32-bit autopull, swizzles pixels to GRBW order, streams 32 bits, and returns HTTP 200 with `{ "status": "ok", "pixels_sent": 1 }`.

#### Scenario: Transmitting APA102 / DotStar 2-wire SPI stream
- **WHEN** client sends `POST /api/v1/bus/led/transmit` with `{ "pin": 10, "clk_pin": 11, "protocol": "spi", "model": "apa102", "brightness": 16, "pixels": [{"r": 255, "g": 128, "b": 0}] }`
- **THEN** server clocks 32-bit zero start frame, LED frame `[0xE0 | 16, B, G, R]`, and end frame clock pulses, returning HTTP 200 with `{ "status": "ok", "pixels_sent": 1, "protocol": "spi" }`.

#### Scenario: Rejecting payloads exceeding 256 pixel limit
- **WHEN** client sends `POST /api/v1/bus/led/transmit` with 300 pixels
- **THEN** server rejects the request with HTTP 400 Bad Request `{ "error": "Maximum strip length is 256 pixels" }`.
