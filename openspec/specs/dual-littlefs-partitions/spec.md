# dual-littlefs-partitions Specification

## Purpose
Provides physical and logical partition isolation in on-probe flash memory between the immutable/read-only System WebUI (`SYSTEM_FS`) and user-installable applications/data (`USER_FS`).

## Requirements

### Requirement: Independent Dual LittleFS Partitions
The firmware SHALL initialize two separate, non-overlapping LittleFS filesystem instances in flash memory above the single application slot: `SYSTEM_FS` at flash `0x00140000` (1 MB, 256 sectors of 4096 bytes, XIP `0x10140000`) and `USER_FS` at flash `0x00240000` (1792 KB, ending at `0x00400000`), falling back gracefully to the In-ROM interactive recovery interface whenever `SYSTEM_FS` is uninitialized.

#### Scenario: Partition boundary mapping
- **WHEN** the storage subsystem initializes at boot
- **THEN** `SYSTEM_FS` is mounted at `0x00140000` with size 1 MB and `USER_FS` is mounted at `0x00240000` with size 1792 KB; the application slot ends at `0x00140000` and is never crossed.

#### Scenario: User partition format isolation
- **WHEN** a user or client issues a format command on user storage
- **THEN** only sectors within `USER_FS` are erased and reformatted, while `SYSTEM_FS` remains completely unmodified and mounted.

#### Scenario: Uninitialized system partition recovery
- **WHEN** `SYSTEM_FS` is unformatted or empty
- **THEN** the HTTP service responds with the In-ROM interactive recovery interface rather than unhandled errors or blank responses.

#### Scenario: OTA never reaches filesystem flash
- **WHEN** firmware OTA writes an image into the single application slot
- **THEN** every erase/write bound check rejects any range at or above `0x00140000`, so neither LittleFS partition can be touched by OTA.
