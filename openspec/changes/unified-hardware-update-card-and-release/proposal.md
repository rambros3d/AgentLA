## Why

AgentLA's initial firmware flashing and update lifecycle currently requires users to juggle multiple manual steps and separate files: an initial bootloader/firmware flash, a secondary WebUI LittleFS UF2 flash, and a standalone recovery page/banner with duplicate buttons and progress bars. Consolidating update and recovery functions directly into a unified hero card on the `[ HARDWARE ]` tab, moving storage partitions and the flash table to a dedicated `[ STORAGE ]` tab, and providing a multi-segment All-in-One UF2 with an automated GitHub Actions release workflow for `v0.1.1` delivers a clean, intuitive, and professional developer experience.

## What Changes

- **Unified System Update Hero Card**: Replace the standalone `#recoveryBanner` and fragmented update buttons with a full-width System Update Hero Card positioned at the top of the `[ HARDWARE ]` tab in `device.html`.
- **Dedicated `[ STORAGE ]` Tab (`/device/storage`)**: Move the storage cards (`WebUI & Storage (SYSTEM_FS)`, `User Storage (USER_FS)`) and the full visual Flash Partitions bar and geometry table into a new dedicated `[ STORAGE ]` tab positioned immediately next to `[ HARDWARE ]`.
- **Elimination of Scary Recovery Concept**: Eliminate `#recoveryBanner`, `#recDropZone`, `#btnRecAuto`, `#recProgress`, and all duplicate `#rec*` elements. When `SYSTEM_FS` is blank or unpopulated, the update card cleanly reports `WebUI: Not Installed` and provides 1-click cloud restoration or local tarball staging.
- **2-Phase Full System Auto Update**: Provide a single `[ ⚡ AUTO UPDATE ALL ]` button that orchestrates Phase 1 (download & stream `webui.tar`) followed by Phase 2 (download & stream `agentla-pico2-ota.bin`) and auto-reboots to activate new firmware.
- **Dynamic LittleFS Auto-Partitioning**: On boot, if `SYSTEM_FS` or `USER_FS` partitions are unformatted, the firmware automatically formats them without requiring external flash partition tools. `USER_FS` dynamically expands across all remaining flash.
- **GitHub Actions Release Pipeline (`.github/workflows/release.yml`)**: Automate building and publishing release artifacts on git tag push (`v*`):
  - `agentla-pico2.uf2`: All-in-One Multi-Segment UF2 containing Bootloader (`0x10000000`), Firmware Slot A (`0x10004000`), and WebUI LittleFS (`0x10204000`).
  - `agentla-pico2-ota.bin`: Packaged OTA firmware with 64-byte `AGLA_FW` header.
  - `webui.tar`: LittleFS WebUI compressed update bundle.
- **Official Repository Redirect**: Update `device.html` release query URL to `api.github.com/repos/rambros3d/AgentLA/releases/latest`.
- **Clean Redirection**: Update `fs_http.rs` to redirect missing HTML page requests directly to `/device`.

## Capabilities

### Modified Capabilities
- `rom-device-diagnostics`: Consolidates release update and recovery management into a top-level Hero Card on the hardware tab, creates a dedicated `[ STORAGE ]` tab (`/device/storage`) for partition cards and flash geometry, and removes obsolete recovery banners and duplicate hooks.
- `live-webui-updates`: Adds 2-phase unified auto-update sequence streaming WebUI before firmware OTA with auto-reboot orchestration.
- `probe-firmware-ota`: Integrates firmware OTA staging with multi-segment All-in-One UF2 delivery and GitHub Actions automated publishing.

## Impact

- **Affected Files**:
  - `firmware-rs/assets/device.html` (Unified Hero Card, new `[ STORAGE ]` tab, removal of `#recoveryBanner`, 2-phase update runner)
  - `firmware-rs/src/web.rs` (Register `/device/storage` and `/device/storage/` routes)
  - `firmware-rs/src/fs_http.rs` (Redirect to `/device`)
  - `.github/workflows/release.yml` (New CI/CD release workflow)
  - `tools/package_uf2.py` (New multi-segment UF2 packager)
  - `test/test_device_diagnostics.py` (Updated assertions for unified hero card, storage tab, and absent recovery banner)
- **Zero Heap / Protocol Integrity**: Firmware remains strictly `#![no_std]` and zero-heap.
