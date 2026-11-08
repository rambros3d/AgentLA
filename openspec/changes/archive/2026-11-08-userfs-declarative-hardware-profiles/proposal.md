## Why

Hardware profiles currently occupy a dedicated 64 KB raw Flash slice (`0x00260000..0x00270000`) formatted as 32 rigid 2 KB binary slots maintaining byte-for-byte legacy C-struct compatibility. This architecture introduces flash wear on every profile write (blocking 4 KB sector erases), limits profiles to 2,000-byte payloads, leaves empty payload metadata `"{}"`, and creates a flash layout gap between firmware slots and LittleFS. Moving profiles into `USER_FS` as declarative JSON files eliminates binary C constraints, leverages LittleFS wear-leveling and crash-safe copy-on-write, allows rich hardware reconfiguration (power rails, pin allocations, instrument defaults), and reclaims the 64 KB slice to expand `USER_FS`.

## What Changes

- **BREAKING**: Remove the 64 KB raw binary flash profile window (`0x00260000..0x00270000`) and legacy C `profile_slot_t` binary packing.
- Shift `SYSTEM_FS` down to `0x00260000..0x002E0000` (fixed 512 KB), immediately adjacent to firmware Slot B.
- Shift `USER_FS` to `0x002E0000..0x003FC000`, expanding by 64 KB (total **1,136 KB** on Pico 2 / 284 blocks; dynamically consuming all remaining flash up to `FLASH_END - 16KB` on BakuProbe).
- Store hardware profiles as human-readable, declarative JSON files inside `USER_FS` under `/profiles/<id>.json`.
- Implement full declarative hardware application on profile activation: quiescing pins, setting target power rail voltage and current trip limits, and pre-claiming pins for designated drivers.
- Implement boot-default resolution via `/profiles/boot_default` file pointer; fall back safely to Safe State (all pins Hi-Z) if `USER_FS` is unformatted or the file is missing.
- Implement automatic ROM template re-seeding (`safe_state.json`, `logic_analyzer.json`, `can_sniffer.json`, `uart_passthrough.json`) if `/profiles` is empty or formatted.
- Update `/api/v1/profiles` REST endpoints to perform atomic LittleFS reads/writes with string `id` parameters instead of numeric slot indices (`0..31`).
- Update `/device` visual partition map to reflect the 5-zone clean partition layout.

## Capabilities

### New Capabilities
- `userfs-declarative-profiles`: Declarative JSON hardware profile engine stored in `USER_FS`, supporting ROM template auto-seeding, boot-default pointer resolution, and atomic LittleFS operations.

### Modified Capabilities
- `hardware-profile-manager`: Replace raw flash sector erase/write and 32-slot C binary records with LittleFS file persistence and active pin/power application.
- `rest-hardware-profiles`: Transition profile endpoints from numeric slot indexes to string IDs and structured JSON payloads.
- `dual-littlefs-partitions`: Shift `SYSTEM_FS` offset to `0x00260000` and expand `USER_FS` offset to `0x002E0000` (+64 KB headroom).
- `rom-device-diagnostics`: Update `/device` visual partition bar and table to reflect the expanded `USER_FS` and removed raw profile slice.

## Impact

- `firmware-rs/src/profile.rs`: Refactored to operate over `USER_FS` LittleFS files; legacy raw flash read/write/erase and binary codecs removed.
- `firmware-rs/src/storage.rs`: Removed `PROFILE_WINDOW_START`, `PROFILE_WINDOW_SIZE`, `profile_flash_*` helpers; updated partition offsets (`SYSTEM_FS_OFFSET = 0x00260000`, `USER_FS_OFFSET = 0x002E0000`).
- `firmware-rs/src/main.rs`: Boot default scan invoked after `storage::init_filesystem()`.
- `firmware-rs/src/rest.rs`: Profile handlers updated for JSON file operations and string IDs.
- `firmware-rs/assets/device.html`: Partition map updated to show `USER_FS` at 1136 KB with no raw `PROF` slice.
- `test/test_hardware_profiles.py`: Updated to validate JSON schema and LittleFS file operations.
