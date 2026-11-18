> **SUPERSEDED / STALE (2026-11-18):** This archived change describes the removed A/B dual-slot boot-table design. The active layout is a single application slot at XIP `0x10008000` with BareBoot RP2350 USB recovery; see `openspec/specs/probe-firmware-ota/spec.md` and `openspec/specs/dual-littlefs-partitions/spec.md`. Kept for history only.

## MODIFIED Requirements

### Requirement: Independent Dual LittleFS Partitions
The firmware SHALL initialize two separate, non-overlapping LittleFS filesystem instances in flash memory placed above the A/B dual firmware slots: Partition 1 (`SYSTEM_FS`) dedicated to the System WebUI and Partition 2 (`USER_FS`) dedicated to user applications and data.

#### Scenario: Partition boundary mapping
- **WHEN** the storage subsystem initializes at boot
- **THEN** `SYSTEM_FS` is mounted at offset `0x00270000` with fixed size `512 KB` (128 sectors of 4096 bytes) and `USER_FS` is mounted at offset `0x002F0000` extending dynamically to `FLASH_END - 16KB`.

#### Scenario: User partition format isolation
- **WHEN** a user or client issues a format command on user storage
- **THEN** only sectors within Partition 2 (`USER_FS`) are erased and reformatted, while Partition 1 (`SYSTEM_FS`) remains completely unmodified and mounted.
