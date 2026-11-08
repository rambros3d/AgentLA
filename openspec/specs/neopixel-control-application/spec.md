# neopixel-control-application Specification

## Purpose
Web desktop application providing multi-protocol addressable LED control, 1D and 2D visualizers, coordinate mapping, 60 FPS FastLED-compatible procedural shaders, and power consumption telemetry.

## Requirements

### Requirement: Neopixel Control Multi-Protocol Configuration
The Neopixel Control application SHALL allow users to select from supported addressable LED protocol chipsets (WS2812, WS2812B, SK6812 RGBW, WS2811, WS2813, WS2815, APA106, APA102, SK9822, WS2801, LPD8806), specify color channel order (RGB, GRB, BGR, RGBW, GRBW), and dynamically configure Data and optional Clock GPIO pins.

#### Scenario: Configuring a 1-wire strip
- **WHEN** user selects `WS2812B` with `GRB` color order and `GPIO 22`
- **THEN** the application dispatches configuration parameters to the hardware and shows a 1-wire pin configuration with the clock selector disabled

#### Scenario: Configuring an APA102 DotStar 2-wire strip
- **WHEN** user selects `APA102` with `BGR` color order, `GPIO 19` for Data, and `GPIO 18` for Clock
- **THEN** the application dispatches configuration parameters to the hardware and enables SPI 2-wire clocked output

### Requirement: Interactive 1D Strip and 2D Matrix Visualizer
The application SHALL provide an interactive visual canvas that renders current pixel colors in real time and allows manual brush painting, color fills, and coordinate mapping for both 1D linear strips/rings and 2D matrix grids with configurable wiring topology (Serpentine, Row-Major, Column-Major).

#### Scenario: Painting pixels on a 1D strip
- **WHEN** user selects a color and clicks or drags across virtual LEDs in the 1D visualizer
- **THEN** the corresponding LEDs update their rendered color immediately and dispatch updated frame buffers to the connected hardware

#### Scenario: Rendering an 8x8 Serpentine Matrix
- **WHEN** user toggles display mode to 2D matrix with dimensions 8×8 and Serpentine wiring
- **THEN** odd rows are mapped in reverse direction so coordinate `(x, y)` corresponds correctly to the physical zigzag LED index

### Requirement: 60 FPS Procedural Shader Engine
The application SHALL provide a client-side procedural animation engine capable of generating dynamic visual patterns (Rainbow Cycle, Cylon Eye, Fire Flicker, Meteor Trail, Breathing, Plasma) and streaming real-time frames at up to 60 FPS.

#### Scenario: Running Rainbow animation
- **WHEN** user selects the Rainbow effect and clicks Start
- **THEN** the browser continuously updates LED hues using `requestAnimationFrame` and streams binary frame updates over the WebSocket connection

### Requirement: Real-Time Power and Current Advisory
The application SHALL compute estimated current draw ($mA$) based on active LED counts, color brightness levels, and supply voltage, displaying a color-coded status badge with safety warnings.

#### Scenario: Operating below USB power limit
- **WHEN** estimated current draw is below 500 mA
- **THEN** the power badge displays green with the calculated milliamps and watts

#### Scenario: Exceeding high current threshold
- **WHEN** estimated current draw exceeds 1500 mA
- **THEN** the power badge displays red with a warning advising external 5V power injection
