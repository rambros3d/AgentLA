# hardware-profile-manager Specification

## Purpose
Manages non-volatile flash-backed hardware configuration profiles, slot persistence, safe handover state transitions, boot profile selection, and WebUI/MCP simulation fidelity.

## Requirements

### Requirement: Declarative Profile Storage in LittleFS
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

### Requirement: WebUI Profile JSON-RPC and Simulation Fidelity
The WebUI Profile Management interface SHALL faithfully support both hardware probe communication and in-browser mock simulation across profile listing, creation, and application.

#### Scenario: Listing hardware profiles via MCP
- **WHEN** the WebUI requests hardware profiles via `tools/call` `profile_list`
- **THEN** the agent returns all configured profile slots with pin assignments and power states.

#### Scenario: In-browser hardware profile simulation
- **WHEN** the WebUI runs in demo or simulation mode without physical hardware
- **THEN** profile transitions simulate realistic pin allocation tables and safe-state resets.

#### Scenario: Saving a profile
- **WHEN** a save profile operation writes a 2 KB sector
- **THEN** interrupts are inhibited only during the direct flash page write and immediately re-enabled.

#### Scenario: Deleting a profile
- **WHEN** a profile slot is erased
- **THEN** sector erase executes cleanly and slot header magic is zeroed.
