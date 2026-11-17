## MODIFIED Requirements

### Requirement: Directory Listing Endpoint
The firmware SHALL provide `GET /api/v1/fs/list?root={system|user}&path={path}` returning a JSON array of entries within the specified directory. The response SHALL contain every entry in the directory as syntactically valid JSON. If the entries cannot be represented within the response buffer, the firmware SHALL return HTTP 500 with `{"error":"listing_too_large"}` rather than a truncated 200 response.

#### Scenario: Listing system_fs directory
- **WHEN** client requests `GET /api/v1/fs/list?root=system&path=/`
- **THEN** firmware accesses `SYSTEM_FS` via `with_system_fs` and returns HTTP 200 with entries array where each object contains `name`, `is_dir`, and `size`.

#### Scenario: Listing user_fs directory
- **WHEN** client requests `GET /api/v1/fs/list?root=user&path=/apps`
- **THEN** firmware accesses `USER_FS` via `with_user_fs` and returns HTTP 200 with directory contents.

#### Scenario: Listing exceeds response capacity
- **WHEN** a directory contains more entries than the response buffer can represent
- **THEN** firmware returns HTTP 500 with `{"error":"listing_too_large"}` and never a partial JSON array.

### Requirement: Mutating Operations with System Partition Protection
The firmware SHALL support write, delete, mkdir, and rename endpoints on `user_fs`, and SHALL reject any mutating operation targeted at `system_fs` with HTTP 403 Forbidden. Writes to `user_fs` SHALL be atomic: the full payload SHALL be staged to a temporary file and the target atomically replaced only after all chunks are persisted, so a failed or interrupted chunk never truncates an existing file.

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

#### Scenario: Interrupted write preserves original file
- **WHEN** a chunk write fails partway through `POST /api/v1/fs/write` for an existing file
- **THEN** the original file remains byte-for-byte intact and the staged temporary file is removed.

## ADDED Requirements

### Requirement: Escaped JSON String Values
The firmware SHALL escape all filesystem-derived strings embedded in JSON responses, including directory entry names and paths echoed by write, delete, mkdir, and rename handlers, so that quotes, backslashes, and control characters cannot corrupt the payload.

#### Scenario: Entry name containing a quote
- **WHEN** a `user_fs` directory contains a file whose name includes a double-quote character
- **THEN** `GET /api/v1/fs/list` returns syntactically valid JSON and the client receives the name unchanged.

#### Scenario: Echoed path containing a quote
- **WHEN** a write, delete, mkdir, or rename operation targets a path containing a double-quote character
- **THEN** the success response is syntactically valid JSON.

### Requirement: Distinct Partition Lock Contention Error
When a filesystem mutex is held by another task, the firmware SHALL report a distinct contention error rather than reporting the partition as unmounted.

#### Scenario: Concurrent filesystem operation
- **WHEN** an FS endpoint is invoked while the target partition mutex is held by another task
- **THEN** the endpoint returns HTTP 503 with `{"error":"fs_busy"}` instead of `{"error":"fs_not_mounted"}`.
