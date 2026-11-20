## ADDED Requirements

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
