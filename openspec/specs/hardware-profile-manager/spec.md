# hardware-profile-manager Specification

## Purpose
Manages non-volatile flash-backed hardware configuration profiles, slot persistence, safe handover state transitions, boot profile selection, and WebUI/MCP simulation fidelity.

## Requirements

### Requirement: 32-Slot Non-Volatile Flash Profile Storage
The firmware SHALL allocate a dedicated 64 KB sector block at the upper boundary of RP2350 flash memory (`PICO_FLASH_SIZE_BYTES - 64KB`) partitioned into 32 individual 2 KB profile slots, with Slot 0 reserved as a read-only factory Safe State.

#### Scenario: Storing a profile in flash
- **WHEN** a client issues a `POST /api/profiles` with a valid JSON workbench state
- **THEN** the firmware SHALL validate the payload, compute its CRC32, write the header and payload into an available flash slot, and return the allocated slot ID.

#### Scenario: Flash persistence across power cycles
- **WHEN** the probe is power-cycled or connected to a different host computer
- **THEN** all saved profiles in flash SHALL remain intact and queryable via `GET /api/profiles`.

#### Scenario: Applying an empty or corrupted slot
- **WHEN** client applies a slot where CRC fails or slot is empty
- **THEN** hardware remains in Safe State and 400 is returned.

### Requirement: Profile Application Handover Sequence
The firmware SHALL execute a safe transition sequence when activating a profile: abort active DMA streams, tri-state all pins to SIO inputs, discharge the target power rail for 5ms, apply new pin claims, and safely re-energize target power.

#### Scenario: Applying a valid profile
- **WHEN** a client issues a `POST /api/profiles/{id}/apply`
- **THEN** the probe SHALL quiesce existing claims, verify pin assignments, apply new driver/observer roles, configure instrument peripherals, and return success.

### Requirement: Boot Default Profile Setting
The firmware SHALL allow marking one user profile slot as the boot default, which auto-applies on startup after the hardware safe-initialization phase.

#### Scenario: Designating a boot profile
- **WHEN** a client issues a `POST /api/profiles/{id}/boot`
- **THEN** the firmware SHALL set the `is_boot` flag for slot `{id}`, clear the flag on all other slots, and update the flash header.

#### Scenario: Clearing boot profile back to Safe State
- **WHEN** a client issues a `POST /api/profiles/0/boot` or clears the boot flag
- **THEN** the firmware SHALL ensure that subsequent power cycles remain strictly in Safe State.

### Requirement: WebUI Profile JSON-RPC and Simulation Fidelity
The WebUI Profile Management interface SHALL faithfully support both hardware probe communication and in-browser mock simulation across profile listing, creation, and application.

#### Scenario: Listing hardware profiles via MCP
- **WHEN** the WebUI requests hardware profiles via `tools/call` `profile_list`
- **THEN** the agent returns all configured profile slots with pin assignments and power states.

#### Scenario: In-browser hardware profile simulation
- **WHEN** the WebUI runs in demo or simulation mode without physical hardware
- **THEN** profile transitions simulate realistic pin allocation tables and safe-state resets.

### Requirement: Slot record binary format parity
The on-flash profile header and record binary representation SHALL maintain byte-exact alignment and CRC32 verification compatibility across C and Rust firmware implementations.

#### Scenario: Record written by Rust readable by external tooling
- **WHEN** a profile is saved to flash
- **THEN** the binary struct layout matches the canonical C struct definition with identical field offsets and CRC32 calculation.

### Requirement: Save and erase are short critical-section bursting
Profile flash erase and program operations SHALL execute within bounded critical sections disabling interrupts only during the minimal SPI write burst, preventing long latency spikes.

#### Scenario: Saving a profile
- **WHEN** a save profile operation writes a 2 KB sector
- **THEN** interrupts are inhibited only during the direct flash page write and immediately re-enabled.

#### Scenario: Deleting a profile
- **WHEN** a profile slot is erased
- **THEN** sector erase executes cleanly and slot header magic is zeroed.
