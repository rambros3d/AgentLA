## Context

Hardware profiles were previously stored in a dedicated 64 KB raw flash slice (`0x00260000..0x00270000`) below `SYSTEM_FS`. The profile engine maintained strict byte-for-byte binary parity with an older C implementation (32 slots of 2 KB each, `magic 0x41474C41`, 32-byte names, CRC32). Each profile write required blocking raw flash sector read-modify-erase-write cycles (30-40 ms), suffered from flash wear vulnerability on power loss, was capped at 2,000 bytes payload, and only contained empty `{}` metadata.

We are dropping backward compatibility entirely, eliminating the raw 64 KB partition, shifting `SYSTEM_FS` directly after firmware Slot B at `0x00260000`, expanding `USER_FS` to `0x002E0000` (+64 KB larger), and implementing declarative JSON profiles on LittleFS under `/profiles/<id>.json`.

## Goals / Non-Goals

**Goals:**
- Eliminate raw flash sector erase/write operations for profiles in favor of LittleFS atomic copy-on-write and wear-leveling.
- Reclaim the 64 KB raw flash slice to increase `USER_FS` capacity to 1,136 KB (+16 blocks) on RP2350 Pico 2 and 13.4+ MB on BakuProbe.
- Define a declarative JSON profile schema that configures power rails (`v_target_mv`, overcurrent limit) and pre-claims GPIO pins for active subsystems (`capture`, `adapter`, `servo`, `pwm`).
- Implement automatic template re-seeding from ROM if `/profiles` is missing or empty.
- Provide clean REST API endpoints for `/api/v1/profiles` using string identifiers.
- Fall back gracefully to Safe State (all pins Hi-Z) if `USER_FS` fails to mount or has no boot-default profile.

**Non-Goals:**
- Backward compatibility with legacy C `profile_slot_t` binary records or `.uf2` dump readers.
- Preserving the numeric 32-slot restriction.
- Complex multi-stage profile transitions; profile apply always passes through intermediate Safe State (quiescing pins, 5 ms rail discharge).

## Decisions

### Decision 1: LittleFS File Storage in `USER_FS` vs `SYSTEM_FS`
- **Choice**: Store profiles in `USER_FS` under `/profiles/<id>.json`.
- **Rationale**: `SYSTEM_FS` is dedicated to WebUI core distribution and is routinely rewritten as a unit when upgrading the WebUI (`POST /api/v1/system/webui/update`). Storing profiles in `USER_FS` ensures WebUI updates do not overwrite custom user profiles.
- **Alternatives considered**:
  - `SYSTEM_FS`: Would risk overwriting user profiles during WebUI tarball extractions.
  - Retaining raw flash window: Causes flash burn-in, blocks concurrent flash access, and wastes 64 KB.

### Decision 2: Declarative Pin & Power Configuration vs Metadata Only
- **Choice**: Full declarative hardware configuration. A profile JSON specifies target rail voltage, overcurrent trip limits, and pin claims.
- **Rationale**: The previous implementation only recorded the active name without acting on hardware. Declarative profiles allow the board to boot directly into a working instrument mode (e.g. 8-channel logic analyzer or CAN sniffer) without requiring manual browser interactions after power-up.

### Decision 3: Boot-Default Resolution via Marker File
- **Choice**: A dedicated marker file `/profiles/boot_default` storing the active profile ID (e.g. `logic_analyzer`).
- **Rationale**: Avoids modifying and rewriting every profile JSON file just to toggle an `is_boot` flag. Setting the boot profile is an atomic single-file write; clearing it is a simple file deletion.
- **Alternatives considered**:
  - Embedding `is_boot: bool` inside each JSON file: Requires scanning and rewriting multiple files when changing the default boot profile.

### Decision 4: ROM Template Auto-Seeding
- **Choice**: Embed static JSON templates in ROM (`safe_state.json`, `logic_analyzer.json`, `can_sniffer.json`, `uart_passthrough.json`). Auto-populate `/profiles` on boot if the directory is missing or empty.
- **Rationale**: Guarantees that formatting `USER_FS` or flashing blank flash always leaves the board in a usable state with working sample configurations.

### Decision 5: Flash Partition Geometry & Offsets
- **Choice**:
  - `BOOT_TABLE`: `0x00000000..0x00004000` (16 KB)
  - `FW_SLOT_A`: `0x00004000..0x00130000` (1200 KB)
  - `FW_SLOT_B`: `0x00130000..0x00260000` (1200 KB)
  - `SYSTEM_FS`: `0x00260000..0x002E0000` (512 KB, 128 blocks)
  - `USER_FS`: `0x002E0000..0x003FC000` (1136 KB, 284 blocks on Pico 2)
  - `TRAIL`: `0x003FC000..0x00400000` (16 KB)

## Risks / Trade-offs

- **[Risk] USER_FS Corruption or Format Wipes Profiles** → **Mitigation**: Factory templates are stored in ROM and automatically re-seeded if `/profiles` is empty. Bootloader falls back cleanly to Safe State (Hi-Z) if the filesystem fails to mount.
- **[Risk] Boot Ordering Changes** → **Mitigation**: `USER_FS` mounts before profile activation in `main.rs`. All pins remain in Hi-Z until `apply_profile` explicitly claims pins.
- **[Risk] Breaking Existing API Clients** → **Mitigation**: User explicitly approved breaking backward compatibility. The REST API is modernized with string IDs and structured JSON.
