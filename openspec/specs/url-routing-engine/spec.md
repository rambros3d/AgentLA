# url-routing-engine Specification

## Purpose
URL routing engine handling factory WebUI serving, root redirection to canonical routes, LittleFS custom application routes, bridge script injection, SPA fallbacks, and REST API dispatch.

## Requirements

### Requirement: HTML Bridge Script Injection
The HTTP server SHALL append `<script src="/agentla-bridge.js" defer></script>` to HTML responses served from LittleFS under `/apps/{app-name}/...` during chunked file streaming.

#### Scenario: Streaming custom app HTML
- **WHEN** a client requests an HTML document under `/apps/{app-name}/`
- **THEN** the server returns HTTP 200 with the bridge script tag appended and matching `Content-Length`.

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

### Requirement: Custom App Routing with Trailing Slash Normalization
The HTTP server SHALL resolve custom user frontends under `/app/{app-name}/` from LittleFS and normalize directory roots.

#### Scenario: Missing trailing slash redirect
- **WHEN** a client requests `http://10.10.10.10/app/can-sniffer`
- **THEN** the server returns HTTP 301 Moved Permanently with `Location: /app/can-sniffer/`.

#### Scenario: Serving custom app index
- **WHEN** a client requests `http://10.10.10.10/app/can-sniffer/`
- **THEN** the server serves `/app/can-sniffer/index.html` from LittleFS with `Content-Type: text/html`.

### Requirement: SPA Routing Fallback
The HTTP server SHALL serve the app's `index.html` when a requested subpath under an installed app does not match an existing file and has no static file extension.

#### Scenario: Client-side routing fallback
- **WHEN** a client requests a non-existent path without file extension under an installed app route
- **THEN** the server serves the app's `index.html` with HTTP 200 OK.

### Requirement: Standalone Device Diagnostics Routing
The HTTP server SHALL route requests for `/device`, `/device/`, and direct requests to `/device.html` to the ROM-embedded `device.html` asset, bypassing LittleFS filesystem lookups.

#### Scenario: Navigating to canonical device route
- **WHEN** a client requests `http://10.10.10.10/device` or `http://10.10.10.10/device.html`
- **THEN** the server returns HTTP 200 OK with `Content-Type: text/html` serving the ROM-embedded diagnostics page.

#### Scenario: Trailing slash redirection for device route
- **WHEN** a client requests `http://10.10.10.10/device/`
- **THEN** the server returns HTTP 301 Moved Permanently redirecting to `/device`.
