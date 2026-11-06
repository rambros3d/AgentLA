## MODIFIED Requirements

### Requirement: Dynamic Pin Arbitration
The HTTP server SHALL provide `POST /api/v1/pins/claim` and `POST /api/v1/pins/release` to allow web apps to request or surrender exclusive ownership of pin groups through the Rust pin-ownership registry. Claims SHALL be atomic (all listed pins claimed or none), SHALL be restricted to the flexible pin pool (GPIO 0–3, 11, 15–29), and SHALL carry a role string mapped to a capability class (e.g. `SIO`, `SPI`, `UART`, `I2C`, `PWM`, `PIO0`, `PIO1`, `PIO2`). Successful claims return a numeric `handle` that is the only valid argument for release.

#### Scenario: Claiming pins for custom peripheral
- **WHEN** client sends `POST /api/v1/pins/claim` with `{ "pins": [15, 16], "owner": "SPI" }`
- **THEN** server checks for conflicts, claims both pins atomically, and returns HTTP 200 with `{ "claimed": true, "pins": [15, 16], "handle": <id> }`

#### Scenario: Conflicting claim attempt
- **WHEN** client sends `POST /api/v1/pins/claim` where any listed pin is not in the flexible pool or is already claimed (including programmer-bridge pins 4–14)
- **THEN** server returns HTTP 409 with a JSON error body naming the offending pins, and none of the requested pins are claimed

#### Scenario: Releasing by handle
- **WHEN** client sends `POST /api/v1/pins/release` with `{ "handle": <id> }`
- **THEN** server releases the claim, quiesces the freed pins to high-Z inputs, and returns HTTP 200 with `{ "released": true, "handle": <id> }`; an unknown or stale handle returns HTTP 404, and a released claim cannot be released again without error
