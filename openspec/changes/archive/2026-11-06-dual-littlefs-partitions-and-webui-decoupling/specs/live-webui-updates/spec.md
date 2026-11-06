## ADDED Requirements

### Requirement: Live WebUI Tarball Update
The firmware SHALL expose a REST endpoint `POST /api/v1/system/webui/update` that accepts a streaming POSIX `ustar` tarball archive and unpacks its files directly into Partition 1 (`SYSTEM_FS`).

#### Scenario: Successful WebUI update
- **WHEN** a client issues `POST /api/v1/system/webui/update` with a valid tarball containing `index.html`, `apps.html`, `style.css`, and `app.js`
- **THEN** the firmware unpacks and writes each file in 512-byte blocks into `SYSTEM_FS` without buffering the entire archive in RAM, returning `200 OK` with JSON summary

#### Scenario: Malformed tarball rejection
- **WHEN** an invalid or truncated tarball stream is uploaded
- **THEN** the unpacker aborts gracefully and returns `400 Bad Request` with an error message, preserving existing valid files
