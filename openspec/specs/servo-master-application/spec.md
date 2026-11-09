# servo-master-application Specification

## Purpose
Interactive multi-channel servo control web application supporting positional and continuous rotation modes, keyframe sequencing, and gamepad input.
## Requirements
### Requirement: Dynamic Multi-Channel Servo Management
The Servo Master application SHALL support dynamic addition, removal, and independent configuration of 1 to 8 servo channels mapped to flexible GPIO pins.

#### Scenario: Adding a servo channel
- **WHEN** user clicks "Add Channel"
- **THEN** a new servo channel card is instantiated with selectable GPIO pin, customizable label, and default calibration parameters.

### Requirement: Dual Positional and Continuous Rotation Modes
Each servo channel in the web interface SHALL support switching between Positional angle control ($0^\circ–180^\circ/270^\circ$) and Continuous Rotation throttle control ($-100\%\dots+100\%$).

#### Scenario: Positional mode angle adjustment
- **WHEN** user moves the angle slider in positional mode
- **THEN** the rotating SVG horn indicator rotates in real time and the client dispatches target updates to `/api/v1/servo/write`.

#### Scenario: Continuous rotation throttle control
- **WHEN** user moves the throttle slider in continuous rotation mode
- **THEN** the animated gear graphic rotates at proportional velocity and the client dispatches speed/direction pulse widths with a center deadband.

### Requirement: Motion Easing and Speed Ramping
The client application SHALL provide software motion smoothing (S-curve interpolation and velocity limits) to prevent abrupt torque spikes and current brownouts.

#### Scenario: Eased transition between distant angles
- **WHEN** user clicks a preset (e.g. from 0° to 180°) with easing enabled
- **THEN** intermediate pulse values are calculated and streamed at 50Hz along an S-curve until the target angle is reached.

### Requirement: Keyframe Pose Sequencer
The application SHALL allow saving snapshots of all active channel positions as named poses and playing back ordered sequences.

#### Scenario: Executing a recorded sequence
- **WHEN** user starts sequence playback
- **THEN** the application transitions through each pose with configured hold and transition durations, looping if enabled.

### Requirement: Gamepad API and Interactive Wiring Guide
The application SHALL interface with connected USB gamepads via the browser Gamepad API and provide an interactive wiring guide modal.

#### Scenario: Gamepad joystick steering
- **WHEN** a connected USB controller thumbstick is moved
- **THEN** the mapped servo channel updates smoothly according to stick deflection.

#### Scenario: Opening wiring guide
- **WHEN** user clicks the wiring guide button
- **THEN** a modal displays proper connections (external 5V supply, shared ground, and signal line) with high inrush current cautions.

### Requirement: DaisyUI and Responsive Layout Integration
The Servo Master application HTML template SHALL link `/daisyui.css` before `/style.css` and use native DaisyUI component classes arranged inside responsive grid containers styled according to the Industrial Design System (`data-style="industrial"`, `data-theme="nothing"` / `industrial-light`, NDot57 typography, dot-grid canvas, and zero emojis). The application SHALL feature a dedicated in-app sub-toolbar beneath `#top-bar` housing Gamepad status, Calibration Zero, and Emergency Disarm controls, and SHALL provide at least 72px bottom clearance padding to avoid occlusion by the host system dock when embedded.

#### Scenario: Rendering servo channels on desktop viewport
- **WHEN** the Servo Master application is displayed on a screen wider than 1024px
- **THEN** servo channel cards display side-by-side in responsive columns within a CSS grid and inherit active industrial system theme tokens.

#### Scenario: Embedded iframe sub-toolbar access
- **WHEN** the Servo Master application is embedded inside the AgentLA OS launcher iframe
- **THEN** Gamepad connection, Zero All, and Emergency Disarm buttons remain accessible in the sub-toolbar while `#top-bar` is suppressed.

