# rest-gpio-pins Specification

## Purpose
Provides complete hardware inspection, direction/pull/value configuration, and arbitration for all 30 GPIO lines over REST.

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
The HTTP server SHALL provide `POST /api/v1/pins/claim` and `POST /api/v1/pins/release` to allow web apps to request or surrender exclusive ownership of pin groups.

#### Scenario: Claiming pins for custom peripheral
- **WHEN** client sends `POST /api/v1/pins/claim` with `{ "pins": [14, 15], "owner": "SPI" }`
- **THEN** server checks for conflicts, claims pins, and returns HTTP 200 with `{ "claimed": true, "pins": [14, 15] }`.
