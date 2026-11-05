## ADDED Requirements

### Requirement: Web desktop shell surface
The firmware SHALL serve the desktop page route from mounted LittleFS or fallback to the embedded page and SHALL expose board settings route.

#### Scenario: REST measure desktop state
- **WHEN** browser calls `/api/settings` without mounted apps
- **THEN** desktop shell responds JSON body and falls back to embedded constants

### Requirement: App launch API contract
The firmware SHALL expose REST contract listing installed apps with entry and size fields (C parity).

#### Scenario: List launching apps
- **WHEN** client GETs `/api/apps` alias
- **THEN** server returns existing apps list with `entry`, `size`, and `path` present
