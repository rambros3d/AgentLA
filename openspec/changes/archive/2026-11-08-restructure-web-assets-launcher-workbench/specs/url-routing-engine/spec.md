## MODIFIED Requirements

### Requirement: Factory WebUI Root Serving Invariant
The HTTP server SHALL serve the factory WebUI workbench (`lab.html`) for requests matching canonical instrument routes: `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, and direct requests to `/lab.html`. Requests to root `/` SHALL be handled by the root redirection handler returning HTTP 302 Found to `default_home_route`.

#### Scenario: Navigating to canonical scope route
- **WHEN** a client requests `http://10.10.10.10/scope` or `http://10.10.10.10/scope/`
- **THEN** the server serves the WebUI workbench (`lab.html`) with HTTP 200 OK.

#### Scenario: Navigating to direct lab.html route
- **WHEN** a client requests `http://10.10.10.10/lab.html`
- **THEN** the server serves the WebUI workbench (`lab.html`) with HTTP 200 OK.

#### Scenario: Root route redirection
- **WHEN** a client requests `http://10.10.10.10/`
- **THEN** the server returns HTTP 302 Found redirecting to the configured `default_home_route` (default: `/home`).

### Requirement: App Launcher Route Resolution
The HTTP server SHALL route requests for `/home`, `/home/`, and direct requests to `/index.html` to the built-in App Launcher catalog (`index.html`) compiled into LittleFS system storage.

#### Scenario: Navigating to launcher URL
- **WHEN** a client requests `http://10.10.10.10/home` or `http://10.10.10.10/home/`
- **THEN** the server serves the App Launcher catalog (`index.html`) with HTTP 200 OK.

#### Scenario: Navigating to index.html route
- **WHEN** a client requests `http://10.10.10.10/index.html`
- **THEN** the server serves the App Launcher catalog (`index.html`) with HTTP 200 OK.
