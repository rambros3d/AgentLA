## 1. Flash Partition Geometry & LittleFS Realignment

- [x] 1.1 Remove raw profile flash window constants (`PROFILE_WINDOW_START`, `PROFILE_WINDOW_SIZE`) and raw erase/write helpers from `firmware-rs/src/storage.rs`
- [x] 1.2 Update partition offsets in `storage.rs`: move `SYSTEM_FS_OFFSET` to `0x00260000` (512 KB, 128 blocks) and `USER_FS_OFFSET` to `0x002E0000` (1136 KB, 284 blocks on Pico 2)
- [x] 1.3 Verify flash mutex and LittleFS storage drivers operate against the updated partition bounds

## 2. Declarative Profile Engine Implementation

- [x] 2.1 Define declarative profile data model in `firmware-rs/src/profile.rs` (id, name, description, power settings, pin claims, instrument configs)
- [x] 2.2 Implement embedded ROM default template assets (`safe_state.json`, `logic_analyzer.json`, `can_sniffer.json`, `uart_passthrough.json`)
- [x] 2.3 Implement LittleFS profile storage operations in `profile.rs` using `with_user_fs` (list, read, write, delete)
- [x] 2.4 Implement ROM template auto-seeding when `/profiles` is missing or empty
- [x] 2.5 Implement boot-default resolution via `/profiles/boot_default` marker file and wire into `firmware-rs/src/main.rs` post-filesystem initialization
- [x] 2.6 Implement full declarative profile application (`apply_profile`): quiesce pins, 5 ms rail settle, configure power rails, and claim pins for drivers

## 3. REST API & Telemetry Integration

- [x] 3.1 Update `GET /api/v1/profiles` in `firmware-rs/src/rest.rs` to serialize existing JSON profiles from `USER_FS`
- [x] 3.2 Update `POST /api/v1/profiles/apply`, `/boot`, `/save`, `/delete` to accept string profile IDs and JSON documents
- [x] 3.3 Ensure active profile status is correctly reflected in `/api/v1/system/telemetry`

## 4. Diagnostics UI & Verification

- [x] 4.1 Update `/device` visual partition bar and geometry table in `firmware-rs/assets/device.html` to reflect the 5-zone layout (1136 KB `USER_FS` at `0x002E0000`)
- [x] 4.2 Update `test/test_hardware_profiles.py` to test JSON profile schema, LittleFS operations, and REST endpoints
- [x] 4.3 Verify compilation with `cargo check` and `cargo build --release` targeting `thumbv8m.main-none-eabihf`
- [x] 4.4 Flash to connected Pico 2 board, format `USER_FS`, verify template auto-seeding and profile application over live HTTP REST
