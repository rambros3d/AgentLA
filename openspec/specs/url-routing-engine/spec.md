# url-routing-engine Specification

## Purpose
TBD - created by archiving change lab-route-and-home-redirect. Update Purpose after archive.
## Requirements
### Requirement: HTML Bridge Script Injection
The HTTP server SHALL append `<script src="/agentla-bridge.js" defer></script>` to HTML responses served from LittleFS under `/apps/{app-name}/...` during chunked file streaming.

#### Scenario: Streaming custom app HTML
- **WHEN** a client requests an HTML document under `/apps/{app-name}/`
- **THEN** the server returns HTTP 200 with the bridge script tag appended and matching `Content-Length`.

### Requirement: Factory WebUI Root Serving Invariant
The HTTP server SHALL serve the factory WebUI workbench from ROM fsdata for requests matching `/lab`, `/lab/`, and `/lab/index.html`. Requests to root `/` SHALL be handled by the root redirection handler rather than directly serving fsdata.

#### Scenario: Navigating to lab route
- **WHEN** a client requests `http://10.10.10.10/lab` or `http://10.10.10.10/lab/` or `http://10.10.10.10/lab/index.html`
- **THEN** the server serves the factory WebUI workbench (`index.html`) from ROM fsdata with HTTP 200 OK.

#### Scenario: Root route redirection
- **WHEN** a client requests `http://10.10.10.10/`
- **THEN** the server returns HTTP 302 Found redirecting to the configured default home route.

### Requirement: Hardware REST and Legacy Alias Routing
The HTTP server routing engine SHALL intercept all requests under `/api/v1/` as well as legacy profile routes `/api/profiles` and `/api/profiles/apply`, dispatching them to `rest_api_handle` before attempting filesystem lookups.

#### Scenario: Dispatching legacy profile route
- **WHEN** client sends a request to `/api/profiles` or `/api/profiles/apply`
- **THEN** `app_routing` matches the URI and forwards the request to `rest_api_handle`, returning appropriate JSON responses with HTTP 200.

#### Scenario: Dispatching hardware instrument REST routes
- **WHEN** client sends a request to `/api/v1/pins`, `/api/v1/bus/i2c/scan`, or `/api/v1/analyzer/status`
- **THEN** `app_routing` routes the request to the REST API engine.

