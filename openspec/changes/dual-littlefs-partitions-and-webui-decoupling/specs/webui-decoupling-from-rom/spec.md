## ADDED Requirements

### Requirement: Dynamic Streaming from System Filesystem
The HTTP web server SHALL stream all Web Desktop assets (`/`, `/index.html`, `/apps`, `/apps.html`, `/style.css`, `/themes.css`, `/themes.json`, `/app.js`, and SVGs) dynamically out of Partition 1 (`SYSTEM_FS`) in chunked HTTP transfers rather than from compiled compile-time ROM string literals.

#### Scenario: Browser requests index.html
- **WHEN** a client performs `GET /` or `GET /index.html`
- **THEN** the server opens `/index.html` in `SYSTEM_FS` and streams its contents in 512-byte chunks with `Content-Type: text/html`

#### Scenario: Browser requests stylesheet
- **WHEN** a client performs `GET /style.css`
- **THEN** the server opens `/style.css` in `SYSTEM_FS` and streams its contents with `Content-Type: text/css`

### Requirement: Concurrent Worker Socket Pool
The HTTP server SHALL maintain at least 4 concurrent listening worker sockets on port 80 to prevent socket exhaustion during parallel asset requests.

#### Scenario: Concurrent asset requests from browser
- **WHEN** a browser opens 4 parallel TCP connections to fetch `index.html`, `style.css`, `themes.css`, and `app.js`
- **THEN** all 4 connections are accepted concurrently by distinct worker sockets without connection refusal or timeout
