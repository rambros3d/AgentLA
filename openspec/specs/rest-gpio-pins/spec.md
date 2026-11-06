# rest-gpio-pins Specification

## Purpose
Provides complete hardware inspection, direction/pull/value configuration, and atomic arbitration for all 30 GPIO lines over REST.

## Requirements

### Requirement: Full 30-Pin State Inspection
The HTTP server SHALL provide `GET /api/v1/pins` returning the real hardware state of all 30 GPIO lines (GPIO 0 to 29) including domain ownership (`PROGRAMMER`, `ADAPTER`, `LOGIC ANALYZER`, `USER_GPIO`), function mode, direction (`in`/`out`), pull state (`none`/`up`/`down`), and current digital level (`0`/`1`).

#### Scenario: Reading complete pin state table
- **WHEN** client sends `GET /api/v1/pins`
- **THEN** server returns HTTP 200 with JSON payload containing `safe` boolean, `active_profile` string, and array of 30 pin state descriptors.

### Requirement: Single-Pin Query and Configuration
The HTTP server SHALL provide `GET /api/v1/pins/{gpio}`, `POST /api/v1/pins/{gpio}/mode`, and `POST /api/v1/pins/{gpio}/write` for targeted inspection and manipulation of individual GPIO pins.

#### Scenario: Inspecting single GPIO pin
- **WHEN** client sends `GET /api/v1/pins/10`
- **THEN** server returns HTTP 200 with `{ "pin": 10, "domain": "ADAPTER", "mode": "I2C", "dir": "out", "val": 1, "pull": "up" }`.

#### Scenario: Setting GPIO pin digital output
- **WHEN** client sends `POST /api/v1/pins/12/write` with `{ "val": 1 }`
- **THEN** server sets GPIO 12 high and returns HTTP 200 with `{ "pin": 12, "val": 1, "status": "ok" }`.

### Requirement: Dynamic Pin Arbitration
The HTTP server SHALL provide `POST /api/v1/pins/claim` and `POST /api/v1/pins/release` to allow web apps to request or surrender exclusive ownership of pin groups through the Rust pin-ownership registry. Claims SHALL be atomic (all listed pins claimed or none), SHALL be restricted to the flexible pin pool (GPIO 0–3, 11, 15–29), and SHALL carry a role string mapped to a capability class (e.g. `SIO`, `SPI`, `UART`, `I2C`, `PWM`, `PIO0`, `PIO1`, `PIO2`). Successful claims return a numeric `handle` that is the only valid argument for release.

#### Scenario: Claiming pins for custom peripheral
- **WHEN** client sends `POST /api/v1/pins/claim` with `{ "pins": [15, 16], "owner": "SPI" }`
- **THEN** server checks for conflicts, claims both pins atomically, and returns HTTP 200 with `{ "claimed": true, "pins": [15, 16], "handle": <id> }`.

#### Scenario: Conflicting claim attempt
- **WHEN** client sends `POST /api/v1/pins/claim` where any listed pin is not in the flexible pool or is already claimed (including programmer-bridge pins 4–14)
- **THEN** server returns HTTP 409 with a JSON error body naming the offending pins, and none of the requested pins are claimed.

#### Scenario: Releasing by handle
- **WHEN** client sends `POST /api/v1/pins/release` with `{ "handle": <id> }`
- **THEN** server releases the claim, quiesces the freed pins to high-Z inputs, and returns HTTP 200 with `{ "released": true, "handle": <id> }`; an unknown or stale handle returns HTTP 404, and a released claim cannot be released again without error.
