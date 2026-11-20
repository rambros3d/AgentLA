## ADDED Requirements

### Requirement: BareFOC Pure-Knob Target Dials and NippleJS Gimbal Joystick
The BareFOC Studio dashboard SHALL integrate `pureknob.min.js` for interactive rotary setpoint dials and `nipplejs.min.js` for 2-DOF differential gimbal pan/tilt joystick control.

#### Scenario: Operating pure-knob target dial in BareFOC Studio
- **WHEN** user drags or scrolls the rotary knob in angle or velocity mode
- **THEN** target setpoint updates smoothly and sends the command over the active transport

#### Scenario: Operating 2-DOF gimbal joystick
- **WHEN** user drags the virtual joystick in the Kinematics panel
- **THEN** Cartesian coordinates $(x, y)$ map to Pitch and Roll gimbal joint angles

### Requirement: BareFOC Multi-Column Widescreen Layout
The BareFOC Studio dashboard and visualizer views SHALL utilize a responsive 12-column widescreen grid (`grid grid-cols-1 lg:grid-cols-12 gap-4 items-stretch`).

#### Scenario: Displaying BareFOC Studio on wide screen
- **WHEN** user loads `/barefoc` on a display $\ge 1024\text{ px}$
- **THEN** Telemetry HUD, controls, and setpoint dials occupy 5 columns, and Vector Scope, Hexagon, and Oscilloscope occupy 7 columns side-by-side
