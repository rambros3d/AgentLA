## MODIFIED Requirements

### Requirement: 32-Slot Non-Volatile Flash Profile Storage
The firmware SHALL allocate a dedicated 64 KiB flash window at offset `0x180000` (directly below the 2 MiB LittleFS partition at `0x190000`) partitioned into 32 individual 2 KiB profile slots, with Slot 0 reserved as a read-only factory Safe State synthesized at read time (never physically stored).

#### Scenario: Storing a profile in flash
- **WHEN** a client issues a `POST /api/v1/profiles/save` with slot, name, and flags
- **THEN** the firmware SHALL write the slot record with a CRC32 payload check into the requested 2 KiB slot and return the slot ID

#### Scenario: Flash persistence across power cycles
- **WHEN** the probe is power-cycled or connected to a different host computer
- **THEN** all saved profiles in flash SHALL remain queryable via `GET /api/v1/profiles`

### Requirement: Profile Application Handover Sequence
The firmware SHALL execute a safe transition sequence when activating a profile: quiesce all flexible pins to high-Z SIO inputs (equivalent of aborting any active claims; no DMA streams exist in the Rust firmware today), settle for 5 ms, and record the applied slot as active. The Rust firmware SHALL NOT attempt to interpret or apply payload contents: profiles remain metadata records (named flags on a profile selector) until a payload schema is defined by a future change.

#### Scenario: Applying a valid profile
- **WHEN** a client issues a `POST /api/v1/profiles/apply` with a valid in-use slot
- **THEN** the firmware SHALL quiesce flexible pins, settle for 5 ms, and report the slot applied as the active profile

#### Scenario: Applying an empty or corrupted slot
- **WHEN** a client issues `POST /api/v1/profiles/apply` with a slot that is not in use or whose record fails integrity checks
- **THEN** the firmware SHALL return an explicit error response and change no pin state

## ADDED Requirements

### Requirement: Slot record binary format parity
The firmware SHALL store each 2 KiB slot record in the C firmware's binary layout: magic `0x41474C41` (`AGLA`), version `1` (u16), flags u16 (bit values: IN_USE = 1, BOOT_DEFAULT = 2, READ_ONLY = 4), 32-byte name, CRC32 over the payload (standard reflected polynomial `0xEDB88320`, init `0xFFFFFFFF`, final xor-out `0xFFFFFFFF`), payload length (u32), and 2000-byte payload field — keeping the flash window inspectable by off-brand tooling that reads the C firmware's slot table.

#### Scenario: Record written by Rust readable by external tooling
- **WHEN** a slot is saved by the Rust firmware and the flash window is read back (e.g. via a `picotool`-adjacent dump)
- **THEN** the record bytes SHALL parse with the C firmware's slot-record binary expectations (magic, version, flags, CRC32 valid over payload)

### Requirement: Save and erase are short critical-section bursting
The firmware SHALL perform profile save (mutate one 2048-byte half of a 4 KiB sector: read-modify-erase-program) and delete (blank one half) through short blocking flash operations while XIP safety discipline is maintained, and SHALL NOT erase any sector at boot except when a boot-default save mutation demands it.

#### Scenario: Saving a profile
- **WHEN** a client saves a profile slot
- **THEN** the firmware performs a single 4 KiB sector erase plus page-granular programming of that sector, with no other flash region touched

#### Scenario: Deleting a profile
- **WHEN** a client deletes an in-use slot
- **THEN** the half-sector becomes blank (0xFF-filled record fields) and the slot no longer reports in-use
