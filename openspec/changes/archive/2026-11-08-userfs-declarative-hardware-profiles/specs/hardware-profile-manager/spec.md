## MODIFIED Requirements

### Requirement: 32-Slot Non-Volatile Flash Profile Storage
The firmware SHALL manage hardware configuration profiles as declarative JSON files within `USER_FS` on LittleFS, with `safe_state` reserved as a read-only factory default.

#### Scenario: Storing a profile in flash
- **WHEN** a client issues a `POST /api/v1/profiles/save` with a valid JSON profile
- **THEN** the firmware validates the payload, writes the document to `/profiles/<id>.json` on `USER_FS`, and returns HTTP 200 OK.

#### Scenario: Flash persistence across power cycles
- **WHEN** the probe is power-cycled or reconnected
- **THEN** all saved profiles in `USER_FS` remain intact and queryable via `GET /api/v1/profiles`.

#### Scenario: Applying an empty or corrupted slot
- **WHEN** client applies a profile that is missing or corrupt
- **THEN** hardware remains in Safe State and HTTP 400 is returned.

### Requirement: Profile Application Handover Sequence
The firmware SHALL execute a safe transition sequence when activating a profile: abort active DMA streams, tri-state all pins to SIO inputs, discharge the target power rail for 5ms, apply target power settings, and claim designated pins for their drivers.

#### Scenario: Applying a valid profile
- **WHEN** a client issues a `POST /api/v1/profiles/apply` with `{"id": "logic_analyzer"}`
- **THEN** the probe quiesces existing claims, discharges rails for 5 ms, claims the requested pins for designated drivers, applies power configuration, and returns HTTP 200 OK.

### Requirement: Boot Default Profile Setting
The firmware SHALL designate the default profile applied at startup via the `/profiles/boot_default` pointer in `USER_FS`.

#### Scenario: Designating a boot profile
- **WHEN** a client issues a `POST /api/v1/profiles/boot` with `{"id": "can_sniffer"}`
- **THEN** the firmware writes `"can_sniffer"` to `/profiles/boot_default` on `USER_FS` and returns HTTP 200 OK.

#### Scenario: Clearing boot profile back to Safe State
- **WHEN** a client issues a `POST /api/v1/profiles/boot` with `{"id": ""}` or `{"id": "safe_state"}`
- **THEN** the firmware removes `/profiles/boot_default` so subsequent boots remain strictly in Safe State.

## REMOVED Requirements

### Requirement: Slot record binary format parity
**Reason**: Legacy C binary struct parity is deprecated in favor of modern declarative JSON profiles in LittleFS.
**Migration**: Use JSON profile documents under `/profiles/<id>.json` in `USER_FS`.

### Requirement: Save and erase are short critical-section bursting
**Reason**: Replaced by LittleFS copy-on-write atomic filesystem writes, eliminating raw SPI flash sector erase bursts.
**Migration**: LittleFS handles transaction safety and wear-leveling across sectors automatically.
