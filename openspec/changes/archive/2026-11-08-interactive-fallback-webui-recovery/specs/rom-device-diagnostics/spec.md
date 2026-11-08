## MODIFIED Requirements

### Requirement: ROM-Embedded Standalone Diagnostics Page
The system SHALL embed the complete HTML, CSS, and JavaScript source of `device.html` directly into the firmware Flash binary via `include_str!` or `include_bytes!`, serving it with `Content-Type: text/html` and `Cache-Control: no-cache, must-revalidate` without accessing LittleFS storage, and cross-linking with the interactive fallback recovery interface.

#### Scenario: Navigating to /device when LittleFS is unformatted or corrupted
- **WHEN** a client navigates to `http://10.10.10.10/device` or `http://10.10.10.10/device.html` while `SYSTEM_FS` is completely unformatted or corrupt
- **THEN** the server returns HTTP 200 OK and serves the complete diagnostics page with intact styling and scripts.

#### Scenario: Navigating from recovery fallback to diagnostics
- **WHEN** user clicks the diagnostics button on the uninitialized partition recovery page
- **THEN** the browser opens `/device` with full hands-free developer tools available.
