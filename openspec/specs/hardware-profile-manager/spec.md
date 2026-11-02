# hardware-profile-manager Specification

## Purpose
TBD - created by archiving change hardware-profiles-and-safe-state. Update Purpose after archive.
## Requirements
### Requirement: 32-Slot Non-Volatile Flash Profile Storage
The firmware SHALL allocate a dedicated 64 KB sector block at the upper boundary of RP2350 flash memory (`PICO_FLASH_SIZE_BYTES - 64KB`) partitioned into 32 individual 2 KB profile slots, with Slot 0 reserved as a read-only factory Safe State.

#### Scenario: Storing a profile in flash
- **WHEN** a client issues a `POST /api/profiles` with a valid JSON workbench state
- **THEN** the firmware SHALL validate the payload, compute its CRC32, write the header and payload into an available flash slot, and return the allocated slot ID

#### Scenario: Flash persistence across power cycles
- **WHEN** the probe is power-cycled or connected to a different host computer
- **THEN** all saved profiles in flash SHALL remain intact and queryable via `GET /api/profiles`

### Requirement: Profile Application Handover Sequence
The firmware SHALL execute a safe transition sequence when activating a profile: abort active DMA streams, tri-state all pins to SIO inputs, discharge the target power rail for 5ms, apply new pin claims, and safely re-energize target power.

#### Scenario: Applying a valid profile
- **WHEN** a client issues a `POST /api/profiles/{id}/apply`
- **THEN** the probe SHALL quiesce existing claims, verify pin assignments, apply new driver/observer roles, configure instrument peripherals, and return success

### Requirement: Boot Default Profile Setting
The firmware SHALL allow marking one user profile slot as the boot default, which auto-applies on startup after the hardware safe-initialization phase.

#### Scenario: Designating a boot profile
- **WHEN** a client issues a `POST /api/profiles/{id}/boot`
- **THEN** the firmware SHALL set the `is_boot` flag for slot `{id}`, clear the flag on all other slots, and update the flash header

#### Scenario: Clearing boot profile back to Safe State
- **WHEN** a client issues a `POST /api/profiles/0/boot` or clears the boot flag
- **THEN** the firmware SHALL ensure that subsequent power cycles remain strictly in Safe State

