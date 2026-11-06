## ADDED Requirements

### Requirement: RESTful Hardware Profile Slot Listing
The HTTP server SHALL provide `GET /api/v1/profiles` (and alias `GET /api/profiles`) returning a JSON array of all 32 hardware profile slots, their slot IDs, names, flags (`in_use`, `is_boot`, `read_only`), and current active profile slot.

#### Scenario: Querying profile slots
- **WHEN** client sends `GET /api/v1/profiles` or `GET /api/profiles`
- **THEN** server returns HTTP 200 with JSON payload containing `active_slot`, `active_name`, and `slots` array.

### Requirement: RESTful Hardware Profile Application
The HTTP server SHALL provide `POST /api/v1/profiles/apply` (and alias `POST /api/profiles/apply`) accepting `{ "slot": <id> }` to apply a saved hardware profile slot immediately.

#### Scenario: Successfully applying profile slot
- **WHEN** client sends `POST /api/v1/profiles/apply` with JSON payload `{ "slot": 1 }`
- **THEN** server applies slot 1 to pin hardware and returns HTTP 200 with `{ "status": "applied", "slot": 1, "name": "I2C Bus Master" }`.

#### Scenario: Applying invalid or empty slot
- **WHEN** client sends `POST /api/v1/profiles/apply` with an empty or corrupted slot ID
- **THEN** server returns HTTP 400 Bad Request with `{ "error": "Invalid profile slot" }`.

### Requirement: RESTful Boot Profile Selection
The HTTP server SHALL provide `POST /api/v1/profiles/boot` accepting `{ "slot": <id> }` to configure the default profile activated at power-on boot.

#### Scenario: Setting boot default profile
- **WHEN** client sends `POST /api/v1/profiles/boot` with `{ "slot": 2 }`
- **THEN** server updates slot 2 header flags to boot default and returns HTTP 200 with `{ "status": "boot_default_set", "slot": 2 }`.
