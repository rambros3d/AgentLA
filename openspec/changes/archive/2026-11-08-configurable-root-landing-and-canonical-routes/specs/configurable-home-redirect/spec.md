## MODIFIED Requirements

### Requirement: Root HTTP 302 Redirection
The HTTP server SHALL intercept requests for root (`/`) and respond with an HTTP 302 Found redirect containing a `Location` header set to the configured `default_home_route` (defaulting to `/home`) and `Cache-Control: no-cache, no-store, must-revalidate`.

#### Scenario: Navigating to root with default settings
- **WHEN** a client issues `GET /` with factory settings
- **THEN** the server returns HTTP 302 with `Location: /home` and empty body.

#### Scenario: Navigating to root with custom configured landing route
- **WHEN** a client issues `GET /` and `default_home_route` is set to `/scope`
- **THEN** the server returns HTTP 302 with `Location: /scope`.

### Requirement: Settings REST API Default Route Configuration
The REST API endpoint `/api/v1/settings` SHALL support reading and updating `default_home_route` and persisting changes to `/system/settings.json` on LittleFS.

#### Scenario: Reading settings
- **WHEN** a client issues `GET /api/v1/settings`
- **THEN** the response JSON includes `"default_home_route": "<route_string>"`.

#### Scenario: Updating settings
- **WHEN** a client issues `POST /api/v1/settings` or `PUT /api/v1/settings` with `{"default_home_route": "/debug"}`
- **THEN** the server updates its in-RAM cached route, persists the configuration to `/system/settings.json` on LittleFS, and returns HTTP 200 OK with `{"status": "ok"}`.

### Requirement: WebUI Landing Page Dropdown
The Settings modal SHALL provide a "Default Landing Page" select dropdown containing canonical system routes: Launcher (`/home`), Scope & Logic Analyzer (`/scope`), Target Debugger (`/debug`), Protocol Adapters (`/adapter`), Pin Matrix (`/pins`), Agent Console (`/agent`), and Serial Terminal (`/terminal`).

#### Scenario: Opening settings modal
- **WHEN** the user opens the Settings modal
- **THEN** the "Default Landing Page" selector displays available canonical routes with the currently configured route selected.

#### Scenario: Changing default landing page
- **WHEN** the user selects `/agent` and saves settings
- **THEN** the client sends a `POST /api/v1/settings` payload updating `default_home_route` to `/agent`.
