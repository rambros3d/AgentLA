# servo-pwm-control Specification

## Purpose
Servo and PWM hardware control engine managing pin arbitration, frequency division, slice ownership, and REST surfaces.

## Requirements

### Requirement: Servo channel claim and attach
The firmware SHALL allow attaching a servo to any flexible pin (GP10..GP29) provided the pin is not already claimed, and SHALL register the claim through `pin_owner` with an explicit owner handle.

#### Scenario: Attaching a servo to a free pin
- **WHEN** a client attaches a servo on a free flexible pin
- **THEN** the pin is claimed with role `PWM_SERVO`, default pulse is set to 1500 µs (center), and success is returned.

#### Scenario: Attaching over an existing claim
- **WHEN** a client attaches a servo on a pin claimed by another peripheral
- **THEN** the request fails with a conflict status and existing hardware configuration is unchanged.

### Requirement: Servo writing semantics
The firmware SHALL support setting servo pulse width directly in microseconds or as an angle (0..180 degrees) mapped through channel calibration endpoints.

#### Scenario: Writing a positional angle
- **WHEN** an angle is written to an attached servo channel
- **THEN** pulse width is mapped to calibrated microsecond range and written to PWM compare register.

#### Scenario: Continuous throttle deadband
- **WHEN** a continuous rotation channel receives a throttle command near zero
- **THEN** the firmware clamps output to neutral deadband pulse to prevent creeping.

### Requirement: Fifty-hertz fast path
For standard 50 Hz RC servos, the divider and wrap registers SHALL be configured for 1 µs timer tick resolution.

#### Scenario: Configuring a 50 Hz servo
- **WHEN** a 50 Hz servo channel is enabled
- **THEN** wrap is set to 19,999 and divider is set to $f_{sys} / 1,000,000$.

### Requirement: Generic frequency path
For arbitrary PWM frequencies, the firmware SHALL compute integer and fractional divider values matching requested frequency within 1%.

#### Scenario: Non-50 Hz frequency set
- **WHEN** a custom PWM frequency (e.g. 1 kHz to 100 kHz) is requested
- **THEN** hardware divider and top wrap registers are updated accordingly.

### Requirement: Slice ownership arbitration to prevent silent clobbering
Because RP2350 PWM slices share frequency dividers between Channel A and Channel B, the engine SHALL enforce frequency compatibility when two pins share a slice.

#### Scenario: Adjacent pins with conflicting slice parameters
- **WHEN** pin A on slice X is configured at 50 Hz and pin B on slice X requests 1 kHz
- **THEN** request is rejected with a frequency conflict error.

### Requirement: Detach and quiesce
Detaching a channel SHALL stop PWM signal generation and release pin claim back to high-impedance safe state.

#### Scenario: Detaching a servo channel
- **WHEN** a client detaches a servo
- **THEN** PWM output stops and pin is returned to unclaimed state.

### Requirement: Unified PWM surface through the same engine
Raw PWM generation and Servo control SHALL share the same hardware slice allocation and pin ownership registry.

#### Scenario: Generic PWM claim and set
- **WHEN** raw PWM is claimed on a free pin
- **THEN** duty cycle and frequency are set without interfering with servo channels.

#### Scenario: PWM claim conflict
- **WHEN** raw PWM attempts to claim a pin currently used by servo
- **THEN** claim fails with conflict.

### Requirement: Servo REST surface
The HTTP REST API SHALL expose endpoints for servo listing, attaching, writing, and detaching.

#### Scenario: Listing servo channels
- **WHEN** `GET /api/v1/servo/channels` is called
- **THEN** list of configured servo channels and current pulse widths is returned.

#### Scenario: Writing an angle over REST
- **WHEN** `POST /api/v1/servo/write` is sent with pin and angle
- **THEN** channel updates and HTTP 200 is returned.

#### Scenario: Writing to an unattached pin
- **WHEN** `POST /api/v1/servo/write` targets a pin with no attached servo
- **THEN** HTTP 404 is returned.
