# dual-littlefs-partitions Specification

## Purpose
Provides physical and logical partition isolation in on-probe flash memory between the immutable/read-only System WebUI (`SYSTEM_FS`) and user-installable applications/data (`USER_FS`).

## Requirements

### Requirement: Independent Dual LittleFS Partitions
The firmware SHALL initialize two separate, non-overlapping LittleFS filesystem instances in flash memory above the 1 MB main application slot: `SYSTEM_FS` at flash `0x00148000` (1 MB, 256 sectors of 4096 bytes, XIP `0x10148000`) and `USER_FS` at flash `0x00248000` (1760 KB, 440 sectors of 4096 bytes, ending at `0x00400000`), falling back gracefully to the In-ROM interactive recovery interface whenever `SYSTEM_FS` is uninitialized.

#### Scenario: Partition boundary mapping
- **WHEN** the storage subsystem initializes at boot
- **THEN** `SYSTEM_FS` is mounted at `0x00148000` with size 1 MB and `USER_FS` is mounted at `0x00248000` with size 1760 KB; the main application slot ends at `0x00148000` and is never crossed.

#### Scenario: User partition format isolation
- **WHEN** a user or client issues a format command on user storage
- **THEN** only sectors within `USER_FS` are erased and reformatted, while `SYSTEM_FS` remains completely unmodified and mounted.

#### Scenario: Uninitialized system partition recovery
- **WHEN** `SYSTEM_FS` is unformatted or empty
- **THEN** the HTTP service responds with the In-ROM interactive recovery interface rather than unhandled errors or blank responses.

#### Scenario: OTA never reaches filesystem flash
- **WHEN** firmware OTA writes an image into the main application slot
- **THEN** every erase/write bound check rejects any range below `0x00048000` or at or above `0x00148000`, so neither the stage-0/recovery images nor either LittleFS partition can be touched by raw OTA writes.

#### Scenario: Staged OTA image in USER_FS
- **WHEN** a client stages a firmware image as the USER_FS file `/ota/update.bin` and reboots
- **THEN** the image is read through the LittleFS API and installed only into the main application slot `[0x48000, 0x148000)`; `SYSTEM_FS` is never modified.
