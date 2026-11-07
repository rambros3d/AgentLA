## Why

Updating the AgentLA probe currently requires manually rebooting into USB BOOTSEL mode and flashing raw binaries via `picotool` or UF2 drag-and-drop. This approach breaks remote headless workflows, terminates the CDC-NCM Ethernet session, and carries significant risk: flashing an oversized or incompatible binary overwrites the profile slots and LittleFS partitions, corrupting web assets and installed test tools. 

To achieve industrial-grade reliability, we introduce a **True A/B Dual-Slot Firmware OTA** architecture. Updates are streamed live over Ethernet directly into the inactive flash slot while the active firmware continues servicing network operations. Once verified, an atomic metadata switch boots the new slot, backed by a hardware watchdog timer that automatically rolls back to the previous known-good firmware if self-tests fail.

## What Changes

- **True A/B Dual-Slot Flash Partitioning**: Reallocates on-probe flash into two symmetric 1.2 MB firmware execution slots (Slot A and Slot B), physically isolating firmware binaries from LittleFS and configuration profiles.
- **OTA Streaming REST API**: Exposes `POST /api/v1/system/firmware/ota` accepting chunked binary streams, writing directly into the inactive slot sectors without touching RAM or LittleFS filesystems.
- **Integrity & Compatibility Validation**: Enforces cryptographic pre-boot checks: image magic header (`AGLA_FW`), target board signature, image size boundary verification (<= 1.2 MB), and SHA-256 / CRC-32 checksums before candidate boot is authorized.
- **Atomic Candidate Boot & Watchdog Rollback**:
  - Writes candidate boot flag and attempt counter into the 16 KB boot metadata sector.
  - Reboots via Cortex-M `AIRCR.SYSRESETREQ`.
  - Inactive slot boots and starts self-tests (clock PLLs, USB descriptors, CDC-NCM networking).
  - If self-tests pass within 15 seconds, the new firmware issues `ota_confirm()` marking the slot permanently active.
  - If the new firmware hangs, crashes, or panics, the RP2350 hardware watchdog triggers a reset, and the boot manager automatically falls back to the previous working slot.
- **Storage Subsystem Adjustment**: Shifts `PROFILE_WINDOW_START` to `0x00260000`, `SYSTEM_FS` to `0x00270000`, and `USER_FS` to `0x002F0000`, guaranteeing that firmware updates can never corrupt LittleFS.

- **Decommissioning BOOTSEL Command & Endpoint**: Completely removes `GET /api/v1/system/bootsel` and the `system_reboot_bootsel` MCP tool. Since OTA is the official mechanism for firmware upgrades, removing runtime BOOTSEL entry eliminates the accidental trigger of ROM bootloader mode that leaves filesystems exposed to blind raw image overwrites.
- **Hardware Verification via Dedicated Debugprobe**: Leverages the attached Raspberry Pi Debugprobe (`2e8a:000c`) wired to the AgentLA RP2350 SWD header for non-invasive hardware validation:
  - Validates boot switcher vectoring and stack setup at `0x10000000` via SWD register reads (`VTOR`, `MSP`, `PC`).
  - Verifies flash partition memory barriers and sector preservation.
  - Flashes the initial 1st-stage bootloader + Slot A image via `probe-rs download --chip RP2350`.
  - Conducts full live OTA stream over CDC-NCM Ethernet (`10.10.10.10`) to Slot B and confirms atomic watchdog swap.

## Capabilities

### New Capabilities
- `probe-firmware-ota`: Dual-slot A/B firmware OTA subsystem providing streaming HTTP updates, image verification, atomic slot switching, and automated hardware watchdog rollback.

### Modified Capabilities
- `dual-littlefs-partitions`: Re-anchors LittleFS `SYSTEM_FS` and `USER_FS` partition boundary offsets to accommodate symmetric 1.2 MB Slot A and Slot B firmware windows.

## Impact

- **Affected Crates**:
  - `crates/agentla-boot-proto`: New shared `#![no_std]` crate defining `BootTable`, `BootState`, `SlotId`, image headers (`AGLA_FW`), and CRC validation.
  - `crates/agentla-bootloader`: New standalone `#![no_std]` 1st-stage boot switcher binary (< 4 KB) residing at flash base `0x00000000`.
  - `firmware-rs`: Updates flash memory geometry in `storage.rs`, consumes `agentla-boot-proto`, adds OTA HTTP streaming endpoint in `web.rs` and `rest.rs`, and implements watchdog self-test confirmation.
  - `boards/pico2` and `boards/bakuprobe`: Updates memory linker scripts (`memory.x`) to reflect the dual-slot execution layout.
- **APIs**:
  - Adds `POST /api/v1/system/firmware/ota` (chunked firmware streaming).
  - Adds `GET /api/v1/system/firmware/status` (reports active slot, candidate slot, version, and rollback state).
  - Adds `POST /api/v1/system/firmware/rollback` (manual force-rollback to alternate slot).
  - Removes `GET /api/v1/system/bootsel` and MCP tool `system_reboot_bootsel`.
