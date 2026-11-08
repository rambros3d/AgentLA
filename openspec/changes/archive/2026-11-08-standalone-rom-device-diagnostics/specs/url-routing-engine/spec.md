## ADDED Requirements

### Requirement: Standalone Device Diagnostics Routing
The HTTP server SHALL route requests for `/device`, `/device/`, and direct requests to `/device.html` to the ROM-embedded `device.html` asset, bypassing LittleFS filesystem lookups.

#### Scenario: Navigating to canonical device route
- **WHEN** a client requests `http://10.10.10.10/device` or `http://10.10.10.10/device.html`
- **THEN** the server returns HTTP 200 OK with `Content-Type: text/html` serving the ROM-embedded diagnostics page.

#### Scenario: Trailing slash redirection for device route
- **WHEN** a client requests `http://10.10.10.10/device/`
- **THEN** the server returns HTTP 301 Moved Permanently redirecting to `/device`.
