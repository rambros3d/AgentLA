## MODIFIED Requirements

### Requirement: RESTful Hardware Profile Slot Listing
The HTTP server SHALL provide `GET /api/v1/profiles` returning a JSON object containing `active_profile`, `boot_profile`, and a `profiles` array of existing profiles on `USER_FS` (with `id`, `name`, `description`, `is_boot`, and `read_only` fields).

#### Scenario: Querying profile slots
- **WHEN** client sends `GET /api/v1/profiles`
- **THEN** server returns HTTP 200 OK with the active profile ID, boot profile ID, and an array of existing profile objects.

#### Scenario: Listing buffer would overflow
- **WHEN** the constructed listing cannot fit its bounded buffer
- **THEN** server returns HTTP 500 with `{"error":"overflow"}` instead of a truncated list.

### Requirement: RESTful Hardware Profile Application
The HTTP server SHALL provide `POST /api/v1/profiles/apply` accepting a JSON body `{"id": "<profile_id>"}` (or `?id=<profile_id>`) to apply a saved profile from `USER_FS`.

#### Scenario: Successfully applying profile slot
- **WHEN** client sends `POST /api/v1/profiles/apply` with `{"id": "can_sniffer"}` and the profile exists
- **THEN** server applies the profile, updates active profile state, and returns HTTP 200 with `{"status":"applied","id":"can_sniffer","name":"..."}`.

#### Scenario: Applying invalid or empty slot
- **WHEN** client sends `POST /api/v1/profiles/apply` with a non-existent profile ID
- **THEN** server returns HTTP 404 `{"error":"profile not found"}` and alters no pin state.

### Requirement: RESTful Boot Profile Selection
The HTTP server SHALL provide `POST /api/v1/profiles/boot` accepting a JSON body `{"id": "<profile_id>"}` (or `?id=<profile_id>`) to designate the startup profile. Providing an empty string or `"safe_state"` removes the boot-default designation.

#### Scenario: Setting boot default profile
- **WHEN** client sends `POST /api/v1/profiles/boot` with `{"id": "logic_analyzer"}`
- **THEN** server writes the boot marker file in `USER_FS` and returns HTTP 200 with `{"status":"boot_default_set","id":"logic_analyzer"}`.

#### Scenario: Clearing boot profile back to Safe State
- **WHEN** client sends `POST /api/v1/profiles/boot` with `{"id": ""}`
- **THEN** server deletes the boot marker file and returns HTTP 200 with `{"status":"boot_default_set","id":""}`.

### Requirement: Profile save and delete endpoints
The HTTP server SHALL provide `POST /api/v1/profiles/save` and `POST /api/v1/profiles/delete` accepting JSON payloads. Save writes or updates `/profiles/<id>.json` on `USER_FS`; delete removes the designated profile file (rejecting attempts to delete read-only templates or Safe State).

#### Scenario: Saving a profile slot
- **WHEN** client sends `POST /api/v1/profiles/save` with a profile JSON body
- **THEN** server writes the file to `USER_FS` and returns HTTP 200 with `{"status":"saved","id":"..."}`.

#### Scenario: Deleting a slot
- **WHEN** client sends `POST /api/v1/profiles/delete` with `{"id": "custom_profile"}`
- **THEN** server deletes the file and returns HTTP 200 with `{"status":"deleted","id":"custom_profile"}`; deleting `safe_state` or a read-only profile returns HTTP 400.
