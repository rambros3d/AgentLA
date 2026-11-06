## MODIFIED Requirements

### Requirement: RESTful Hardware Profile Slot Listing
The HTTP server SHALL provide `GET /api/v1/profiles` (and alias `GET /api/profiles`) returning a JSON object with `active_slot`, `active_name`, and a `slots` array of all 32 slots (ID, name, `in_use`, `is_boot`, `read_only` flags; slot 0 reported as the read-only Safe State). The response SHALL be constructed in a bounded buffer sized for the full 32-slot listing, and SHALL return an explicit HTTP 500 `{"error":"overflow"}` response rather than silently truncating the array.

#### Scenario: Querying profile slots
- **WHEN** client requests `GET /api/v1/profiles`
- **THEN** server returns HTTP 200 with the complete `active_slot` / `active_name` / 32-slot listing (every in-use slot present)

#### Scenario: Listing buffer would overflow
- **WHEN** the constructed listing cannot fit its bounded buffer
- **THEN** server returns HTTP 500 with `{"error":"overflow"}` instead of a truncated list

### Requirement: RESTful Hardware Profile Application
The HTTP server SHALL provide `POST /api/v1/profiles/apply` (and alias `POST /api/profiles/apply`) accepting `?slot=<id>` or a JSON body `{ "slot": <id> }` to apply a saved profile slot. Applicable statuses: success reports `{"status":"applied","slot":N,"name":"…"}`, invalid or empty slot returns HTTP 400 `{"error":"invalid slot"}`, out-of-range (not 0..32) slot returns HTTP 400, and a slot that is not in use returns HTTP 404 `{"error":"slot not in use"}`.

#### Scenario: Successfully applying profile slot
- **WHEN** client sends `POST /api/v1/profiles/apply?slot=1` (or JSON body `{ "slot": 1 }`) and slot 1 is in use
- **THEN** server applies slot 1 as the active profile and returns HTTP 200 with `{ "status": "applied", "slot": 1, "name": "…" }`

#### Scenario: Applying invalid or empty slot
- **WHEN** client sends `POST /api/v1/profiles/apply` with an empty, corrupted, or not-in-use slot ID
- **THEN** server returns an explicit 400 or 404 JSON error body and changes no pin state

### Requirement: RESTful Boot Profile Selection
The HTTP server SHALL provide `POST /api/v1/profiles/boot` accepting `?slot=<id>` or a JSON body `{ "slot": <id> }` to configure the default profile activated at power-on boot. Slot 0 or an explicit clear SHALL remove the boot-default designation from every slot (ensure power cycles boot into Safe State).

#### Scenario: Setting boot default profile
- **WHEN** client sends `POST /api/v1/profiles/boot` with `{ "slot": 2 }` and slot 2 is in use
- **THEN** server updates slot 2 header flags to `BOOT_DEFAULT`, clears the flag on every other slot, and returns HTTP 200 with `{ "status": "boot_default_set", "slot": 2 }`

#### Scenario: Clearing boot profile back to Safe State
- **WHEN** client sends `POST /api/v1/profiles/boot` with `{ "slot": 0 }`
- **THEN** server ensures no slot carries `BOOT_DEFAULT` and returns HTTP 200 with `{ "status": "boot_default_set", "slot": 0 }`, and subsequent power cycles boot with no profile applied

## ADDED Requirements

### Requirement: Profile save and delete endpoints
The HTTP server SHALL provide `POST /api/v1/profiles/save` and `POST /api/v1/profiles/delete` (and alias `POST /api/profiles/delete`) accepting slot and (for save) name/`is_boot` parameters via query or JSON body.

#### Scenario: Saving a profile slot
- **WHEN** client sends `POST /api/v1/profiles/save?slot=3&name=MyProfile` (or JSON body)
- **THEN** server writes the slot record (payload stays metadata-only, currently the literal `{}` as in the C firmware) and returns HTTP 200 with `{ "status": "saved", "slot": 3, "name": "MyProfile" }`

#### Scenario: Deleting a slot
- **WHEN** client sends `POST /api/v1/profiles/delete?slot=3` with slot 3 in use
- **THEN** server blanks the slot record and returns HTTP 200 with `{ "status": "deleted", "slot": 3 }`; deleting slot 0 or a `READ_ONLY` slot returns HTTP 400

#### Scenario: Errors
- **WHEN** save/delete targets a nonexistent (blank) record or an out-of-range slot
- **THEN** server returns an explicit 400/404 JSON error without hardware side effects
