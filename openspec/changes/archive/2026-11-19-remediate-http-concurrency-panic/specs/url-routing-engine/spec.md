## ADDED Requirements

### Requirement: Panic-Free Header and Route Dispatch
The URL routing engine and request parsing pipeline SHALL safely parse HTTP headers and dispatch matched routes without raising slice indexing out-of-bounds panics or stack overflow exceptions.

#### Scenario: Root route dispatch
- **WHEN** a client performs `GET /` with standard HTTP headers
- **THEN** the server returns HTTP 302 Found redirecting to `default_home_route` without firmware crash.

#### Scenario: BareFOC canonical route dispatch
- **WHEN** a client performs `GET /barefoc`
- **THEN** the server returns HTTP 200 with `barefoc.html` without firmware crash.
