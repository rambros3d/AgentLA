## Why

With AgentLA's multi-app LittleFS storage, URL routing engine, and App Store in place, users need tangible, high-value flagship applications that showcase the flexibility and responsiveness of user-installable web frontends. Hobby RC servos and continuous rotation actuators are ubiquitous in robotics, mechatronics, pan/tilt gimbals, and automated test fixtures, but controlling them requires microsecond-accurate PWM pulses and an intuitive multi-channel control interface.

Introducing the "Servo Master" example application and high-resolution microsecond PWM backend demonstrates how easily AgentLA can transform into an interactive multi-axis actuation workbench with zero additional firmware flashing.

## What Changes

- **High-Resolution Microsecond PWM Engine**: Upgrade firmware PWM control from coarse percentage duty cycles to exact $1\ \mu\text{s}$ hardware timer resolution ($f_{\text{sys}} = 150\text{ MHz}$, clock divider $= 150.0$, 50 Hz frame period).
- **Dual-Layer REST API Endpoints**:
  - `/api/v1/pwm`: Low-level high-resolution PWM endpoints (`claim`, `set`, `release`, and channel status).
  - `/api/v1/servo`: Dedicated servo control endpoints (`attach`, `write`, `write_us`, `detach`, and multi-channel telemetry).
- **First-Boot App Seeding**: Stage the compiled `servo-master` app into ROM fsdata and automatically seed it into the LittleFS flash partition on first boot, making it instantly available in "My Apps" at `http://10.10.10.10/apps/servo-master/`.
- **Servo Master Application (`/apps/servo-master/`)**:
  - **Dynamic Multi-Channel Support**: Assign 1 to 8 independent servo channels to any flexible GPIO pin (GP10–GP29).
  - **Positional & Continuous Rotation Modes**: Supports standard $0^\circ–180^\circ$ positional servos with real-time rotating SVG horn indicators, and $360^\circ$ continuous rotation servos with bidirectional throttle sliders, deadband calibration, and instant braking.
  - **Motion Easing & Pose Sequencer**: Software S-curve transition smoothing to eliminate mechanical jerk and current surges; keyframe pose snapshotting and looped playback.
  - **Browser Gamepad API Integration**: Native USB/Bluetooth joystick and gamepad support (Xbox/PS4/flight controllers) mapped directly to servo axes.
  - **Interactive Wiring & Safety Guide**: Built-in modal diagram illustrating external 5V/6V power supplies and common-ground connections.

## Capabilities

### New Capabilities
- `high-res-pwm-signal-generation`: High-resolution microsecond pulse width generation (1 µs hardware tick resolution) via RP2350 PWM slices.
- `servo-pwm-rest-api`: Dedicated REST endpoints under `/api/v1/pwm` and `/api/v1/servo` for pulse width and servo angle control.
- `servo-master-application`: Dedicated multi-channel web application installed at `/apps/servo-master/` supporting positional angles, 360° continuous rotation throttle, live rotating SVG horn, S-curve software easing, keyframe pose sequencer, and browser Gamepad API integration.
- `first-boot-app-seeding`: Firmware mechanism to automatically seed default starter applications into LittleFS partition upon first boot or format.

### Modified Capabilities

## Impact

- **Firmware Size**: Minimal impact (~2 KB flash for PWM divider math and REST dispatcher).
- **Hardware Integration**: Dynamically reserves and releases pins from `pin_owner` flexible pool (GP10–GP29) with emergency safe state integration.
- **LittleFS**: Consumes ~80 KB of LittleFS flash space for the seeded `servo-master` app bundle.
