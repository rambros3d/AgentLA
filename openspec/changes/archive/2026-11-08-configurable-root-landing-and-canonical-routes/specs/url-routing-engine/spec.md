## MODIFIED Requirements

### Requirement: Factory WebUI Root Serving Invariant
The HTTP server SHALL serve the factory WebUI workbench (`index.html`) for requests matching canonical instrument routes: `/scope`, `/debug`, `/adapter`, `/pins`, and `/agent`. Requests to root `/` SHALL be handled by the root redirection handler returning HTTP 302 Found to `default_home_route`.

#### Scenario: Navigating to canonical scope route
- **WHEN** a client requests `http://10.10.10.10/scope` or `http://10.10.10.10/scope/`
- **THEN** the server serves the WebUI workbench (`index.html`) with HTTP 200 OK.

#### Scenario: Root route redirection
- **WHEN** a client requests `http://10.10.10.10/`
- **THEN** the server returns HTTP 302 Found redirecting to the configured `default_home_route` (default: `/home`).

### Requirement: App Launcher Route Resolution
The HTTP server SHALL route requests for `/home` and `/home/` to the built-in App Launcher page (`apps.html`) compiled into LittleFS system storage.

#### Scenario: Navigating to launcher URL
- **WHEN** a client requests `http://10.10.10.10/home` or `http://10.10.10.10/home/`
- **THEN** the server serves the App Launcher dashboard (`apps.html`) with HTTP 200 OK.

## REMOVED Requirements

### Requirement: Legacy Route Aliases
**Reason**: Backward compatibility is not needed; legacy aliases (`/lab`, `/apps`, `/target`, `/console`, `/pin`) clutter route tables and documentation.  
**Migration**: Clients must navigate to canonical routes: `/home`, `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, and `/terminal`.
