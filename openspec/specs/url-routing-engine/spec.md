# url-routing-engine Specification

## Purpose
URL routing engine handling factory WebUI serving, root redirection, LittleFS custom application routes, bridge script injection, SPA fallbacks, and REST API dispatch.

## Requirements

### Requirement: HTML Bridge Script Injection
The HTTP server SHALL append `<script src="/agentla-bridge.js" defer></script>` to HTML responses served from LittleFS under `/apps/{app-name}/...` during chunked file streaming.

#### Scenario: Streaming custom app HTML
- **WHEN** a client requests an HTML document under `/apps/{app-name}/`
- **THEN** the server returns HTTP 200 with the bridge script tag appended and matching `Content-Length`.

### Requirement: Factory WebUI Root Serving Invariant
The HTTP server SHALL serve the factory WebUI workbench from ROM fsdata for requests matching `/lab`, `/lab/`, and `/lab/index.html`. Requests to root `/` SHALL be handled by the root redirection handler or default landing route.

#### Scenario: Navigating to lab route
- **WHEN** a client requests `http://10.10.10.10/lab` or `http://10.10.10.10/lab/` or `http://10.10.10.10/lab/index.html`
- **THEN** the server serves the factory WebUI workbench (`index.html`) from ROM fsdata with HTTP 200 OK.

#### Scenario: Root route redirection
- **WHEN** a client requests `http://10.10.10.10/`
- **THEN** the server returns HTTP 302 Found redirecting to the configured default home route.

#### Scenario: Navigating to root URL
- **WHEN** a client requests `http://10.10.10.10/` or `http://10.10.10.10/index.html`
- **THEN** the server serves the default factory WebUI from ROM fsdata with HTTP 200 OK.

### Requirement: App Launcher Route Resolution
The HTTP server SHALL route requests for `/apps` and `/apps/` to the built-in App Launcher page compiled into ROM fsdata.

#### Scenario: Navigating to launcher URL
- **WHEN** a client requests `http://10.10.10.10/apps` or `http://10.10.10.10/apps/`
- **THEN** the server serves the App Launcher dashboard from ROM fsdata with HTTP 200 OK.

### Requirement: Custom App Routing with Trailing Slash Normalization
The HTTP server SHALL resolve custom user frontends under `/apps/{app-name}/` from LittleFS and normalize directory roots.

#### Scenario: Missing trailing slash redirect
- **WHEN** a client requests `http://10.10.10.10/apps/can-sniffer`
- **THEN** the server returns HTTP 301 Moved Permanently with `Location: /apps/can-sniffer/`.

#### Scenario: Serving custom app index
- **WHEN** a client requests `http://10.10.10.10/apps/can-sniffer/`
- **THEN** the server serves `/apps/can-sniffer/index.html` from LittleFS with `Content-Type: text/html`.

### Requirement: SPA Routing Fallback
The HTTP server SHALL serve the app's `index.html` when a requested subpath under an installed app does not match an existing file and has no static file extension.

#### Scenario: Client-side routing path request
- **WHEN** a client requests `http://10.10.10.10/apps/can-sniffer/dashboard/filters` and no such file exists on flash
- **THEN** the server serves `/apps/can-sniffer/index.html` with HTTP 200 OK.

#### Scenario: Missing static asset 404
- **WHEN** a client requests `http://10.10.10.10/apps/can-sniffer/assets/missing.png` and the file does not exist
- **THEN** the server returns HTTP 404 Not Found.

### Requirement: Hardware REST and Legacy Alias Routing
The HTTP server routing engine SHALL intercept all requests under `/api/v1/` as well as legacy profile routes `/api/profiles` and `/api/profiles/apply`, dispatching them to `rest_api_handle` before attempting filesystem lookups.

#### Scenario: Dispatching legacy profile route
- **WHEN** client sends a request to `/api/profiles` or `/api/profiles/apply`
- **THEN** `app_routing` matches the URI and forwards the request to `rest_api_handle`, returning appropriate JSON responses with HTTP 200.

#### Scenario: Dispatching hardware instrument REST routes
- **WHEN** client sends a request to `/api/v1/pins`, `/api/v1/bus/i2c/scan`, or `/api/v1/analyzer/status`
- **THEN** `app_routing` routes the request to the REST API engine.
