## ADDED Requirements

### Requirement: Hardware REST and Legacy Alias Routing
The HTTP server routing engine SHALL intercept all requests under `/api/v1/` as well as legacy profile routes `/api/profiles` and `/api/profiles/apply`, dispatching them to `rest_api_handle` before attempting filesystem lookups.

#### Scenario: Dispatching legacy profile route
- **WHEN** client sends a request to `/api/profiles` or `/api/profiles/apply`
- **THEN** `app_routing` matches the URI and forwards the request to `rest_api_handle`, returning appropriate JSON responses with HTTP 200.

#### Scenario: Dispatching hardware instrument REST routes
- **WHEN** client sends a request to `/api/v1/pins`, `/api/v1/bus/i2c/scan`, or `/api/v1/analyzer/status`
- **THEN** `app_routing` routes the request to the REST API engine.
