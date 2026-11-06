## ADDED Requirements
### Requirement: Desktop shell surface
The system SHALL respond on `/` with the desktop page (embedded or mounted).
#### Scenario: Get desktop
- **WHEN** GET `/`
- **THEN** returns 200 HTML
### Requirement: Board settings API
The system SHALL expose `/api/settings` (stub JSON)
#### Scenario: settings
- **WHEN** GET `/api/settings`
- **THEN** returns 200 JSON
### Requirement: Board manifest API
The system SHALL expose `/api/board/manifest`
#### Scenario: manifest
- **WHEN** GET `/api/board/manifest`
- **THEN** returns 200 JSON
