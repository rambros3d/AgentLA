## ADDED Requirements

### Requirement: Servo channel claim and attach
The engine SHALL manage up to 8 concurrent servo channels keyed by GPIO within the flexible pin pool, each secured through the pin-ownership registry as a `GPIO_FUNC_PWM` driver claim. Attach SHALL configure limits clamped into `[400,1500]` µs minimum and `[1500,3000]` µs maximum (defaults 500/2500 when omitted), center the pulse at 1500 µs, and initialize the channel to positional mode at 90° (continuous mode, if requested, reports angle −100).

#### Scenario: Attaching a servo to a free pin
- **WHEN** a client attaches a servo to an unclaimed flexible pin with optional `min_us`/`max_us`/`continuous` parameters
- **THEN** the engine claims the pin as a PWM driver, configures the slice, and responds reporting the effective limits and initial center pulse

#### Scenario: Attaching over an existing claim
- **WHEN** a client attaches a servo to a pin either already held by a servo channel or by another pin-owner claimant
- **THEN** the engine SHALL return HTTP 409 conflict with a JSON error body and SHALL NOT modify the slice or pin configuration

### Requirement: Servo writing semantics
The engine SHALL provide `write_angle` (positional 0–180° mapped linearly from `min_us` to `max_us`, or continuous throttle −100..+100 with a ±5 deadband mapping reverse range `min_us..1480` µs and forward `1520..max_us`), and `write_us` (direct pulse width clamped into `[min_us, max_us]`). Continuously updated pulse SHALL reach the output at the next repeating period without glitching between writes.

#### Scenario: Writing a positional angle
- **WHEN** `write_angle` is called with an angle within 0..180 for a positional channel
- **THEN** the generated pulse width equals `min_us + angle × (max_us − min_us) / 180` µs

#### Scenario: Continuous throttle deadband
- **WHEN** `write_angle` is called with an absolute throttle value of 5 or less on a continuous channel
- **THEN** the channel output SHALL be driven at its neutral pulse (within the 1480–1520 µs center band)

### Requirement: Fifty-hertz fast path
For a 50 Hz channel, the engine SHALL configure the slice with a 1 µs effective tick at the 150 MHz system clock (integer clock divide of 150), period wrap of 19999, and duty level equal to the requested pulse microseconds.

#### Scenario: Configuring a 50 Hz servo
- **WHEN** a channel is attached or updated at frequency 50 with pulse 1500 µs
- **THEN** the slice register state reflects clock-divider 150, wrap 19999, and level 1500

### Requirement: Generic frequency path
For frequencies other than 50 Hz (clamped 1..100000 Hz), the engine SHALL compute `div = sys_clk / (freq × 65536)` clamped 1..255 and `wrap = sys_clk / (freq × div) − 1` clamped 1..65534, and scale the pulse level proportionally to the reduced wrap period.

#### Scenario: Non-50 Hz frequency set
- **WHEN** a pin is set to frequency F with pulse P µs via the generic path
- **THEN** the slice clock-divider, wrap, and level values SHALL be computed from F and the 150 MHz system clock, and the effective output SHALL have period 1/F with duty corresponding to P µs

### Requirement: Slice ownership arbitration to prevent silent clobbering
The engine SHALL track ownership of each RP2350 PWM slice (2 channels per slice) and SHALL refuse configuration changes to a slice whose other channel is owned by a different claimant with inconsistent parameters rather than silently rewriting shared divider/wrap registers.

#### Scenario: Adjacent pins with conflicting slice parameters
- **WHEN** a client sets the frequency or pulse of one channel of a shared slice whose sibling channel belongs to an active conflicting configuration
- **THEN** the engine SHALL return a conflict error and SHALL NOT mutate the shared slice registers (unlike the C firmware, which silently rewrote them)

### Requirement: Detach and quiesce
The engine SHALL detach a channel by disabling slice output, returning the pin to SIO input (high-Z, pulls none), and releasing its pin-ownership claim. Quiesce-all during profile transitions SHALL release every owned channel identically.

#### Scenario: Detaching a servo channel
- **WHEN** a client detaches an active channel
- **THEN** the pin SHALL be returned to high-Z input, slice ownership cleared, and subsequent rotate/writes to that pin SHALL fail with a not-attached error

### Requirement: Unified PWM surface through the same engine
The engine SHALL additionally expose generic PWM duty/frequency operation (`/api/v1/pwm/**`) for pins not carrying a servo attach, using the same slice-ownership table; a pin owned by either surface SHALL be interchangeable only via explicit claim and release operations on the same engine.

#### Scenario: Generic PWM claim and set
- **WHEN** a client claims a free pin via `POST /api/v1/pwm/claim` and sets frequency and pulse width via `POST /api/v1/pwm/set`
- **THEN** the slice is configured for the requested continuous pulse/frequency output and reports the channel in `GET /api/v1/pwm`

#### Scenario: PWM claim conflict
- **WHEN** a client claims a pin already owned as a servo channel or otherwise claimed
- **THEN** the server returns HTTP 409 with a JSON error and makes no hardware changes

### Requirement: Servo REST surface
The HTTP server SHALL provide `GET /api/v1/servo` returning active channels with `pin`, `angle`, `pulse_us`, `min_us`, `max_us`, and `continuous` fields, and `POST /api/v1/servo/attach`, `/servo/write`, `/servo/write_us`, and `/servo/detach` accepting pin and channel parameters as query arguments or JSON body, with responses matching the C firmware shapes (`{"status":"attached"|"ok"|"detached", ...}`).

#### Scenario: Listing servo channels
- **WHEN** a client sends `GET /api/v1/servo`
- **THEN** the server returns HTTP 200 with a JSON object listing each active channel and its state

#### Scenario: Writing an angle over REST
- **WHEN** a client sends `POST /api/v1/servo/write` with a pin carrying an active positional channel
- **THEN** the server updates the channel and returns HTTP 200 with the resulting status object

#### Scenario: Writing to an unattached pin
- **WHEN** a client sends `POST /api/v1/servo/write` for a pin with no active servo channel
- **THEN** the server returns HTTP 404 with a JSON error body
