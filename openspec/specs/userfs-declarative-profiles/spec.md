# UserFS Declarative Hardware Profiles

## Purpose
Hardware profiles define electrical power rails, pin allocations, and instrument configurations stored as declarative JSON documents inside `USER_FS`.

## Requirements

### Requirement: Declarative JSON Profile Storage in LittleFS
The system SHALL store hardware profiles as individual JSON files in `USER_FS` within the `/profiles/` directory, identified by human-readable string IDs (`/profiles/<id>.json`).

#### Scenario: Storing a profile file
- **WHEN** a client saves a profile with id `"can_sniffer"`
- **THEN** the firmware writes `/profiles/can_sniffer.json` to `USER_FS` with copy-on-write atomic filesystem semantics.

#### Scenario: Listing available profiles
- **WHEN** a client queries existing profiles
- **THEN** the firmware iterates entries in `/profiles/` and parses metadata without requiring raw sector scans.

### Requirement: Automatic ROM Template Re-Seeding
The system SHALL inspect `/profiles` upon mounting `USER_FS` during boot. If `/profiles` does not exist or contains no profiles, the firmware SHALL automatically re-seed factory default template files from ROM.

#### Scenario: Fresh boot after partition format
- **WHEN** `USER_FS` is formatted or brand new
- **THEN** the firmware automatically generates `/profiles/` and seeds `safe_state.json`, `logic_analyzer.json`, `can_sniffer.json`, and `uart_passthrough.json`.

### Requirement: Boot-Default Profile Resolution via Marker File
The system SHALL determine the power-on boot profile using a dedicated marker file `/profiles/boot_default` containing the designated profile ID string.

#### Scenario: Boot with configured default profile
- **WHEN** the board boots and `/profiles/boot_default` exists with content `"logic_analyzer"`
- **THEN** the firmware applies the `"logic_analyzer"` profile after filesystem initialization.

#### Scenario: Boot fallback without default profile
- **WHEN** `/profiles/boot_default` is missing or `USER_FS` is unmountable
- **THEN** the firmware remains strictly in Safe State (all pins Hi-Z, power rails deactivated).

### Requirement: Full Declarative Profile Hardware Handover
When applying a profile, the system SHALL quiesce all flexible pin claims, discharge target power rails for 5 ms, parse the profile JSON, apply target power rail configuration, and claim pins for specified subsystem drivers.

#### Scenario: Activating a declarative profile
- **WHEN** a profile specifies `v_target_mv: 3300` and pin 4 as `adapter`
- **THEN** pins quiesce to Hi-Z for 5 ms, the target power rail configures to 3.3V, and pin 4 is claimed by the adapter driver.
