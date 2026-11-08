## ADDED Requirements

### Requirement: Storage Statistics Endpoint
The firmware SHALL provide `GET /api/v1/fs/stat` returning JSON-encoded storage metrics for both `SYSTEM_FS` and `USER_FS`.

#### Scenario: Querying storage stats
- **WHEN** client sends `GET /api/v1/fs/stat`
- **THEN** firmware returns HTTP 200 with JSON payload containing `{ "system": { "total_bytes": u32, "used_bytes": u32, "read_only": true }, "user": { "total_bytes": u32, "used_bytes": u32, "read_only": false } }`.

### Requirement: Directory Listing Endpoint
The firmware SHALL provide `GET /api/v1/fs/list?root={system|user}&path={path}` returning a JSON array of entries within the specified directory.

#### Scenario: Listing system_fs directory
- **WHEN** client requests `GET /api/v1/fs/list?root=system&path=/`
- **THEN** firmware accesses `SYSTEM_FS` via `with_system_fs` and returns HTTP 200 with entries array where each object contains `name`, `is_dir`, and `size`.

#### Scenario: Listing user_fs directory
- **WHEN** client requests `GET /api/v1/fs/list?root=user&path=/apps`
- **THEN** firmware accesses `USER_FS` via `with_user_fs` and returns HTTP 200 with directory contents.

### Requirement: File Streaming Read Endpoint
The firmware SHALL provide `GET /api/v1/fs/read?root={system|user}&path={path}` that streams file bytes in 512-byte LittleFS chunks with appropriate MIME headers.

#### Scenario: Reading file from system_fs
- **WHEN** client requests `GET /api/v1/fs/read?root=system&path=/themes.json`
- **THEN** firmware streams the file contents in chunked transfer encoding with `Content-Type: application/json`.

#### Scenario: Reading non-existent file
- **WHEN** client requests a path that does not exist in the selected partition
- **THEN** firmware returns HTTP 404 Not Found.

### Requirement: Mutating Operations with System Partition Protection
The firmware SHALL support write, delete, mkdir, and rename endpoints on `user_fs`, and SHALL reject any mutating operation targeted at `system_fs` with HTTP 403 Forbidden.

#### Scenario: Writing to user_fs
- **WHEN** client sends `POST /api/v1/fs/write?root=user&path=/profiles/custom.json` with file payload
- **THEN** firmware writes the data to `USER_FS` in 512-byte blocks and returns HTTP 200.

#### Scenario: Attempting write to system_fs
- **WHEN** client sends `POST /api/v1/fs/write?root=system&path=/index.html`
- **THEN** firmware rejects the request with HTTP 403 Forbidden and error message `system_fs is read-only`.

#### Scenario: Deleting entry from user_fs
- **WHEN** client sends `DELETE /api/v1/fs/entry?root=user&path=/temp.txt`
- **THEN** firmware deletes the file from `USER_FS` and returns HTTP 200.

#### Scenario: Attempting delete from system_fs
- **WHEN** client sends `DELETE /api/v1/fs/entry?root=system&path=/logo.svg`
- **THEN** firmware rejects the request with HTTP 403 Forbidden.

#### Scenario: Creating directory in user_fs
- **WHEN** client sends `POST /api/v1/fs/mkdir?root=user&path=/newdir`
- **THEN** firmware creates the directory in `USER_FS` and returns HTTP 200.

#### Scenario: Renaming entry in user_fs
- **WHEN** client sends `POST /api/v1/fs/rename?root=user&src=/old.txt&dst=/new.txt`
- **THEN** firmware renames the item in `USER_FS` and returns HTTP 200.
