## 1. High-Resolution Microsecond PWM Engine

- [x] 1.1 Implement microsecond PWM divider and wrap calculations ($f_{\text{sys}} / 1,000,000$, 50 Hz base, 1 tick = $1\ \mu\text{s}$)
- [x] 1.2 Update signal generator `cmd_out_set` and add `pulse_us` hardware configuration support
- [x] 1.3 Implement multi-channel servo state tracking (up to 8 channels) with calibration limits (`min_us`, `max_us`, `center_us`)
- [x] 1.4 Wire PWM channels to dynamic pin ownership manager with safe-state auto-quiescing

## 2. REST API Endpoints (`/api/v1/pwm` and `/api/v1/servo`)

- [x] 2.1 Implement `POST /api/v1/pwm/claim` and `POST /api/v1/pwm/set` with `pulse_us` and `freq`
- [x] 2.2 Implement `POST /api/v1/pwm/release` and `GET /api/v1/pwm` status handler
- [x] 2.3 Implement `POST /api/v1/servo/attach`, `POST /api/v1/servo/write` (angle mapping), and `POST /api/v1/servo/write_us`
- [x] 2.4 Implement `POST /api/v1/servo/detach` and `GET /api/v1/servo` multi-channel telemetry handler

## 3. First-Boot Starter App Seeding

- [x] 3.1 Implement seed extractor function in `littlefs_driver.c` to seed `/apps/servo-master/` if missing on boot
- [x] 3.2 Stage `servo-master` assets (`app.json`, `index.html`) into ROM fsdata build pipeline
- [x] 3.3 Verify automatic seeding on clean LittleFS format

## 4. Servo Master Web Application UI (`/apps/servo-master/`)

- [x] 4.1 Create `firmware/html/apps/servo-master/index.html` with responsive DaisyUI/Tailwind bento aesthetic
- [x] 4.2 Create `app.json` manifest with icon, category, and metadata
- [x] 4.3 Implement dynamic channel cards (1 to 8 channels) with GPIO pin selector, channel labeling, and remove buttons
- [x] 4.4 Implement Positional Mode with angle slider ($0^\circ–180^\circ$), presets ($0^\circ, 45^\circ, 90^\circ, 180^\circ$), and rotating SVG horn indicator
- [x] 4.5 Implement Continuous Rotation Mode with center-sprung throttle (-100% to +100%), deadband calibration, and spinning gear animation
- [x] 4.6 Implement per-channel calibration modal (Min $\mu s$, Max $\mu s$, Invert Direction)

## 5. Advanced Motion & Input Features

- [x] 5.1 Implement client-side S-curve motion easing and velocity limits for smooth angle transitions
- [x] 5.2 Implement Keyframe Pose Sequencer (save poses, build ordered timeline, play/loop/stop)
- [x] 5.3 Implement HTML5 Gamepad API polling loop with axis-to-channel mapping
- [x] 5.4 Implement interactive wiring guide and power advisory modal (5V/6V external supply & common ground)

## 6. Automated Testing & Verification

- [x] 6.1 Create test script `test/test_servo_master_app.py` validating PWM microsecond math, REST endpoints, and app seeding
- [x] 6.2 Verify clean firmware compilation and test execution
