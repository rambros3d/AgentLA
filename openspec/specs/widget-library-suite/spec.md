# widget-library-suite Specification

## Purpose
TBD - created by archiving change webapp-widgets-and-widescreen-layout. Update Purpose after archive.
## Requirements
### Requirement: Canonical In-ROM Widget Micro-Libraries
The system SHALL provide canonical, zero-dependency widget micro-libraries as shared in-ROM assets in `webui/` and serve them from the `SYSTEM_FS` partition:
- `pureknob.min.js`: Canvas2D rotary knobs, dials, and radial gauges.
- `nipplejs.min.js`: 2D virtual analog joystick and XY touchpad.
- `ogl.min.js`: Minimalist WebGL 3D engine with native Quaternion and Euler math.
- `agentla-led-view.js`: Addressable LED visualizer for 1D strips, 2D matrices, and circular rings.
- `agentla-servo-view.js`: Parametric servo horn visualizer with travel arcs and timeline keyframing.
- `agentla-thermal-view.js`: Bilinear interpolation heatmap engine for AMG8833/MLX90640 thermal cameras.
- `agentla-pinout-view.js`: Interactive 40-pin RP2350 header visualizer with Pin Arbiter badges.
- `agentla-storage-map.js`: LittleFS flash sector grid visualizer with wear-leveling counts.

#### Scenario: Client requests widget library asset
- **WHEN** client requests `GET /pureknob.min.js`, `GET /nipplejs.min.js`, or `GET /ogl.min.js`
- **THEN** server responds with HTTP 200, Content-Type `application/javascript`, and gzip compression

#### Scenario: Size budget invariant
- **WHEN** all widget micro-libraries are compressed into `build/webui.tar`
- **THEN** the total compressed size of the new widget files SHALL be less than 40 KB

### Requirement: Zero Emoji and Zero CSS Dot Indicator Compliance
All widget scripts, rendered labels, and visual outputs SHALL strictly maintain zero emoji characters and zero CSS dot indicators (`.active::after`).

#### Scenario: Verifying widget text labels
- **WHEN** widget components render status, units, or numeric labels
- **THEN** output text contains only standard ASCII technical labels and SVG icons without emojis

