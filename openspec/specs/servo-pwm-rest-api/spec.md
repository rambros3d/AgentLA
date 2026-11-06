# servo-pwm-rest-api Specification

## Purpose
RESTful HTTP API endpoints for high-resolution PWM configuration and multi-channel servo actuation.

## Requirements

### Requirement: Low-Level High-Resolution PWM REST Endpoints
The REST API SHALL provide endpoints under `/api/v1/pwm` to claim, configure, and release high-resolution PWM channels with explicit microsecond pulse widths.

#### Scenario: Setting PWM pulse width
- **WHEN** client sends `POST /api/v1/pwm/set` with parameters `pin=16&freq=50&pulse_us=1500`
- **THEN** server configures the hardware PWM slice and returns HTTP 200 with `{ "status": "ok", "pin": 16, "freq": 50, "pulse_us": 1500 }`.

#### Scenario: Querying PWM channel telemetry
- **WHEN** client sends `GET /api/v1/pwm`
- **THEN** server returns HTTP 200 with JSON array of all active PWM channels, their frequencies, and current pulse widths.

### Requirement: Dedicated Servo Control REST Endpoints
The REST API SHALL provide endpoints under `/api/v1/servo` to attach, write angles, write raw pulse widths, and detach servos.

#### Scenario: Attaching a servo
- **WHEN** client sends `POST /api/v1/servo/attach` with parameters `pin=16&min_us=500&max_us=2500`
- **THEN** server claims the pin, initializes 50Hz PWM, centers the servo at $1500\ \mu\text{s}$, and returns HTTP 200.

#### Scenario: Writing servo angle
- **WHEN** client sends `POST /api/v1/servo/write` with parameters `pin=16&angle=90`
- **THEN** server calculates the calibrated pulse width and returns HTTP 200 with `{ "status": "ok", "pin": 16, "angle": 90, "pulse_us": 1500 }`.

#### Scenario: Detaching a servo
- **WHEN** client sends `POST /api/v1/servo/detach` with parameter `pin=16`
- **THEN** server disables PWM, releases the pin claim to high-Z, and returns HTTP 200.
