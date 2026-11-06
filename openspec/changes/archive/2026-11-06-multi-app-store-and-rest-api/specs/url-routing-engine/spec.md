## ADDED Requirements

### Requirement: Factory WebUI Root Serving Invariant
The HTTP server SHALL always route root requests (`/`, `/index.html`, `/style.css`, `/app.js`, `/favicon.svg`) to the built-in factory ROM fsdata regardless of LittleFS state.

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
