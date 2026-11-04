## Context

AgentLA runs on the RP2350 microcontroller and now supports custom user-installable web frontends hosted under `/apps/{app-name}/` in LittleFS. To demonstrate the power, responsiveness, and flexibility of this architecture, we are building "Servo Master": an interactive multi-axis actuation application for hobby RC servos, pan/tilt gimbals, robotic arms, and continuous rotation mechanisms.

Controlling standard hobby servos requires a $50\text{ Hz}$ ($20\text{ ms}$) pulse-width-modulated (PWM) signal where the high pulse width ($500\ \mu\text{s}$ to $2500\ \mu\text{s}$) determines angular position or rotational velocity. Previously, AgentLA's signal generator accepted only coarse integer duty percentages (0–100%), which yielded only 5 coarse positions across the entire servo travel. This design upgrades the PWM engine to native $1\ \mu\text{s}$ hardware resolution and adds dedicated REST endpoints alongside the full-featured Servo Master frontend.

## Goals / Non-Goals

**Goals:**
- Provide $1\ \mu\text{s}$ pulse width precision on RP2350 hardware PWM slices for smooth, jitter-free servo motion.
- Implement both general `/api/v1/pwm` and dedicated `/api/v1/servo` REST endpoints.
- Support 1 to 8 simultaneous servo channels across flexible GPIOs (GP10–GP29).
- Support both Positional ($0^\circ–180^\circ/270^\circ$) and Continuous Rotation ($360^\circ$ bidirectional throttle) modes.
- Implement software S-curve motion easing and keyframe pose sequencing in the web client.
- Support USB/Bluetooth gamepads via HTML5 Gamepad API for joystick control.
- Automatically seed `servo-master` into LittleFS on first boot, making it ready out of the box in "My Apps".

**Non-Goals:**
- Driving high-power brushless gimbal motors without ESCs (this app targets standard 3-wire hobby servos / PWM ESCs).
- Implementing closed-loop encoder PID control inside firmware (open-loop pulse width control).

## Decisions

### 1. High-Resolution Microsecond PWM Clock Math
- **Decision**: Configure the RP2350 PWM slice divider dynamically based on `clk_sys`:
  - $\text{div} = f_{\text{sys}} / 1,000,000.0$ (at $150\text{ MHz}$, $\text{div} = 150.0$).
  - $\text{wrap} = 20,000 - 1 = 19,999$ ($50\text{ Hz}$ frame rate).
  - Channel compare level: $\text{level} = \text{pulse\_us}$ ($1\ \mu\text{s}$ per tick).
- **Rationale**: Setting the counter tick to exactly $1.0\ \mu\text{s}$ simplifies calculations, eliminates rounding errors, and ensures zero timing jitter.

### 2. Dual-Layer REST API (`/api/v1/pwm` and `/api/v1/servo`)
- **Decision**: Expose:
  - Low-level `/api/v1/pwm/set?pin={pin}&freq={hz}&pulse_us={us}` for general electronics use.
  - High-level `/api/v1/servo/write?pin={pin}&angle={deg}` and `/api/v1/servo/write_us?pin={pin}&pulse_us={us}` with automatic angle-to-microsecond calibration mapping.
- **Rationale**: Keeps the firmware API general and reusable while giving servo-focused applications maximum convenience.

### 3. First-Boot LittleFS Seeding
- **Decision**: Compile the `servo-master` application into ROM fsdata as a starter template. On initial boot (or if `/apps/servo-master` is missing in LittleFS), the firmware extracts and seeds `/apps/servo-master/index.html` and `app.json` into LittleFS.
- **Rationale**: Gives users an immediate out-of-the-box working application without requiring internet access or manual file uploads, while still allowing the user to modify or delete it from flash.

### 4. Client-Side Software Easing & Speed Limiting
- **Decision**: Implement S-curve easing and speed limiting in client-side JavaScript. Instead of sending an instantaneous 90-degree step change, intermediate pulse targets are interpolated at 50 Hz.
- **Rationale**: Large instantaneous angle jumps draw high inrush currents (up to 2A per servo), which can brown out USB rails or strip plastic gears. Software ramping provides smooth cinematic motion.

### 5. Multi-Mode Architecture (Positional + Continuous Rotation)
- **Decision**: Each channel can be independently toggled between:
  - *Positional*: $0^\circ–180^\circ/270^\circ$ with rotating SVG horn indicator and angle presets.
  - *Continuous Rotation*: Bidirectional throttle slider (-100% to +100%) with calibrated deadband ($1480–1520\ \mu\text{s}$) and spinning gear animation.
- **Rationale**: Broadens the app's utility to motorized camera sliders, conveyor belts, and robot wheel drives.

### 6. HTML5 Gamepad API Integration
- **Decision**: Poll `navigator.getGamepads()` during the 50 Hz animation frame loop, mapping analog axes to selected servo channels.
- **Rationale**: Provides an intuitive physical control interface for multi-joint robot arms and pan/tilt cameras with zero software drivers.

## Risks / Trade-offs

- **[Risk] High inrush current causes USB brownout or device reset** → *Mitigation*: Prominent warning banner and interactive wiring guide detailing external 5V/6V power supplies; software motion ramping prevents sudden stalls.
- **[Risk] Rapid slider movement overwhelms HTTP connection pool** → *Mitigation*: Client throttles HTTP POST requests to 50 Hz (1 request every 20ms) and uses abort controller or inflight request coalescing.
- **[Risk] Slices sharing pins overwrite wrap frequencies** → *Mitigation*: All hobby servos share the standard $50\text{ Hz}$ wrap period. Pins on the same slice (Channels A and B) share wrap seamlessly with independent pulse levels.
