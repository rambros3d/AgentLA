## ADDED Requirements

### Requirement: File manager routes and asset serving
The HTTP server SHALL serve `files.html` on routes `/files` and `/files.html` from the `SYSTEM_FS` partition with transparent gzip streaming when available.

#### Scenario: Requesting /files
- **WHEN** client sends `GET /files`
- **THEN** server responds with HTTP 200 and Content-Type `text/html`.

#### Scenario: Requesting /files.html
- **WHEN** client sends `GET /files.html`
- **THEN** server responds with HTTP 200 and Content-Type `text/html`.
