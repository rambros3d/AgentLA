# native-neopixel-system-app Specification

## Purpose
TBD - created by archiving change native-neopixel-system-app. Update Purpose after archive.
## Requirements
### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/neopixel.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving neopixel asset from SYSTEM_FS
- **WHEN** client requests `GET /neopixel.html` or `GET /neopixel`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/neopixel` to `SystemWebFile::new("neopixel.html", "text/html")` and provide an HTTP redirect for `/led`.

#### Scenario: Accessing canonical neopixel route
- **WHEN** user navigates to `/neopixel`
- **THEN** server responds with HTTP 200 and serves NeoPixel Control

#### Scenario: Accessing neopixel alias route
- **WHEN** user navigates to `/led`
- **THEN** server responds with HTTP 301 redirect to `/neopixel`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `NeoPixel` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope`, `/terminal`, `/barefoc`, `/foc`, or `/servo`
- **THEN** the top navigation rail displays a link to `NeoPixel` (`/neopixel`)

#### Scenario: Active state styling on neopixel page
- **WHEN** user is viewing `/neopixel`
- **THEN** the NeoPixel navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration with Solar Duotone Icon
The system SHALL integrate the NeoPixel launcher into the floating glass dock and application launcher drawer using the Solar Bold Duotone Filters SVG icon.

#### Scenario: Launching from dock
- **WHEN** user clicks the NeoPixel icon in the floating dock
- **THEN** the system navigates directly to `/neopixel`

#### Scenario: Visual icon appearance
- **WHEN** dock is rendered
- **THEN** NeoPixel app icon displays three overlapping optical filter circles using SVG opacity duotone styling

### Requirement: WebUI Packaging Inclusion
The build utility `tools/pack_webui.py` SHALL include `neopixel.html` when packaging the `SYSTEM_FS` image and update tarballs.

#### Scenario: Packaging SYSTEM_FS
- **WHEN** `python3 tools/pack_webui.py` is executed
- **THEN** `neopixel.html` is validated and packed into `build/webui.img` and `build/webui.tar`

### Requirement: Standardized Neopixel LED Visualizer Engine
The NeoPixel Control application SHALL integrate `agentla-led-view.js` as its primary visualizer engine for 1D linear strips, 2D matrix grids, and circular rings.

#### Scenario: Switching topology to 2D matrix or ring
- **WHEN** user selects Matrix or Ring mode in NeoPixel Control
- **THEN** visualizer renders the LEDs in the selected geometry with bloom shaders and real-time power dissipation

### Requirement: NeoPixel Control Multi-Column Widescreen Layout
The NeoPixel Control page SHALL arrange lighting pattern controls, color palette pickers, and power HUD in a multi-column side-by-side grid with the LED simulation canvas.

#### Scenario: Widescreen studio view
- **WHEN** user views NeoPixel Control on a wide screen
- **THEN** pattern generators and color controls occupy 4 columns, and the interactive LED canvas occupies 8 columns

