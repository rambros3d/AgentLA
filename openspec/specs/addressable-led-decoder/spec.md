## ADDED Requirements

### Requirement: Single-Wire PWM Addressable LED Decoding
The firmware SHALL decode single-wire PWM/NRZ addressable LED bitstreams (WS2812B, WS2811, SK6812, WS2813, WS2815, UCS1903) based on high-pulse duration ($T_{\text{high}}$) and reset LOW gap detection ($>50\,\mu\text{s}$).

#### Scenario: Decoding WS2812B 24-bit GRB pixels
- **WHEN** an 800 kHz bitstream containing 3 pixels with color `0xFF0055` (Red=255, Green=0, Blue=85) is captured
- **THEN** the decoder decodes each pixel into RGB hex `#FF0055` and maps pixels to indices 0, 1, and 2

#### Scenario: Decoding SK6812 32-bit RGBW pixels
- **WHEN** an SK6812 bitstream containing 32 bits per pixel with dedicated White channel (R:0, G:128, B:255, W:64) is captured
- **THEN** the decoder extracts RGB `#0080FF` and White channel value 64 for each pixel

### Requirement: Two-Wire Clocked SPI LED Decoding
The firmware SHALL decode two-wire synchronous SPI addressable LEDs (APA102, DotStar, SK9822, WS2801) using clock edges, extracting 5-bit Global Brightness ($0-31$) and 24-bit color data.

#### Scenario: Decoding APA102 with global brightness
- **WHEN** an APA102 stream with 32-bit Start Frame (`0x00000000`) and LED word `0xEF0000FF` (Brightness 15, Blue 0, Green 0, Red 255) is captured
- **THEN** the decoder extracts brightness level 15/31 and color `#FF0000`

### Requirement: LED Timing Jitter and Violation Diagnostics
The decoder SHALL measure high-pulse width on each bit and flag a timing warning if $T_{\text{high}}$ exceeds $1.4\,\mu\text{s}$ or if a reset gap occurs prematurely.

#### Scenario: Detecting LED bit timing violation
- **WHEN** a pulse on a WS2812 line remains high for $1.8\,\mu\text{s}$
- **THEN** the decoder logs an `LED_TIMING_VIOLATION` event with the measured pulse width
