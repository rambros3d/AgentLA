# web-desktop-daisyui-shell Specification

## Purpose
DaisyUI desktop shell surface, board settings, and board manifest API endpoints.

## Requirements

### Requirement: Desktop shell surface
The system SHALL respond on `/` with the desktop page (embedded or mounted).

#### Scenario: Get desktop
- **WHEN** GET `/`
- **THEN** returns 200 HTML.

### Requirement: Board settings API
The system SHALL expose `/api/settings` (stub JSON).

#### Scenario: settings
- **WHEN** GET `/api/settings`
- **THEN** returns 200 JSON.

### Requirement: Board manifest API
The system SHALL expose `/api/board/manifest`.

#### Scenario: manifest
- **WHEN** GET `/api/board/manifest`
- **THEN** returns 200 JSON.

### Requirement: File manager routes and asset serving
The HTTP server SHALL serve `files.html` on routes `/files` and `/files.html` from the `SYSTEM_FS` partition with transparent gzip streaming when available.

#### Scenario: Requesting /files
- **WHEN** client sends `GET /files`
- **THEN** server responds with HTTP 200 and Content-Type `text/html`.

#### Scenario: Requesting /files.html
- **WHEN** client sends `GET /files.html`
- **THEN** server responds with HTTP 200 and Content-Type `text/html`.
