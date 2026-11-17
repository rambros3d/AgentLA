## ADDED Requirements

### Requirement: MCP NeoPixel Fill Tool
The system SHALL expose `neopixel_fill` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Filling pixels with a color
- **WHEN** client invokes `tools/call` with name `neopixel_fill`, `pin`, `color`, and `count`
- **THEN** system drives the first `count` pixels to `color` and returns success

### Requirement: MCP NeoPixel Set Tool
The system SHALL expose `neopixel_set` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Setting a single pixel at index
- **WHEN** client invokes `tools/call` with name `neopixel_set`, `pin`, `index`, and `color`
- **THEN** system updates the pixel at `index` to `color` and returns success

### Requirement: MCP NeoPixel Clear Tool
The system SHALL expose `neopixel_clear` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Clearing pixels
- **WHEN** client invokes `tools/call` with name `neopixel_clear`, `pin`, and `count`
- **THEN** system blanks all `count` pixels to zero (black/off) and returns success

### Requirement: MCP NeoPixel Pattern Tool
The system SHALL expose `neopixel_pattern` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Running procedural pattern
- **WHEN** client invokes `tools/call` with name `neopixel_pattern`, `pin`, `pattern`, `count`, and optional `speed`
- **THEN** system executes the hardware pattern generator and returns status

### Requirement: MCP NeoPixel Analyze Tool
The system SHALL expose `neopixel_analyze` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Measuring signal timing
- **WHEN** client invokes `tools/call` with name `neopixel_analyze` and `pin`
- **THEN** system measures WS2812 pulse widths and returns timing margins

### Requirement: MCP NeoPixel Transmit Tool
The system SHALL expose `neopixel_transmit` in `tools/list` and dispatch it in `tools/call`.

#### Scenario: Transmitting raw color array
- **WHEN** client invokes `tools/call` with name `neopixel_transmit`, `pin`, and `colors`
- **THEN** system transmits the color array to the LED strip and returns count of pixels updated
