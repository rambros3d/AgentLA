# dual-littlefs-partitions Specification

## Purpose
Provides physical and logical partition isolation in on-probe flash memory between the immutable/read-only System WebUI (`SYSTEM_FS`) and user-installable applications/data (`USER_FS`).

## Requirements

### Requirement: Independent Dual LittleFS Partitions
The firmware SHALL initialize two separate, non-overlapping LittleFS filesystem instances in flash memory placed above the A/B dual firmware slots: Partition 1 (`SYSTEM_FS`) dedicated to the System WebUI and Partition 2 (`USER_FS`) dedicated to user applications and data.

#### Scenario: Partition boundary mapping
- **WHEN** the storage subsystem initializes at boot
- **THEN** `SYSTEM_FS` is mounted at offset `0x00270000` with fixed size `512 KB` (128 sectors of 4096 bytes) and `USER_FS` is mounted at offset `0x002F0000` extending dynamically to `FLASH_END - 16KB`

#### Scenario: User partition format isolation
- **WHEN** a user or client issues a format command on user storage
- **THEN** only sectors within Partition 2 (`USER_FS`) are erased and reformatted, while Partition 1 (`SYSTEM_FS`) remains completely unmodified and mounted
