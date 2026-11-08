## Context

AgentLA is an all-in-one embedded hardware diagnostics, scope, and analyzer probe built on the RP2350 microcontroller. Currently, the Flash ROM console (`/device`) contains fragmented update controls across separate cards, plus a dedicated `#recoveryBanner` when `SYSTEM_FS` is blank or unpopulated. Additionally, flashing a bare board required separate flashing steps for firmware and LittleFS filesystem images, and release creation on GitHub lacked an automated CI/CD pipeline.

## Goals / Non-Goals

**Goals:**
- Provide a unified, full-width System Update Hero Card (`#cardSystemUpdate`) at the top of the `[ HARDWARE ]` tab in `device.html`.
- Create a dedicated `[ STORAGE ]` tab (`/device/storage`) hosting the LittleFS partition cards and visual flash memory map.
- Eliminate `#recoveryBanner` and all redundant `#rec*` elements, handling unpopulated WebUI state cleanly within the unified update card.
- Implement an automated 2-phase client update runner (`Auto Update All`) streaming `webui.tar` followed by `agentla-pico2-ota.bin` and warm reboot.
- Provide a multi-segment All-in-One UF2 packaging utility (`tools/package_uf2.py`) combining Bootloader (`0x10000000`), Firmware Slot A (`0x10004000`), and WebUI LittleFS (`0x10204000`) for 1-step USB BOOTSEL flashing.
- Provide an automated GitHub Actions release workflow (`.github/workflows/release.yml`) targeting tag pushes (`v*`) to `rambros3d/AgentLA`.

**Non-Goals:**
- Modifying the `#![no_std]` streaming OTA endpoints (`POST /api/v1/system/firmware/ota` and `POST /api/v1/system/webui/update`).
- Changing the partition boundary offsets (`0x00000000` bootloader, `0x00004000` Slot A, `0x00104000` Slot B, `0x00204000` SYSTEM_FS).

## Decisions

### 1. Tab Organization: 5 Dedicated Views
The navigation bar is organized into 5 clearly scoped views:
```
[ HARDWARE ]  [ STORAGE ]  [ PIN DIAGNOSTICS ]  [ REST API ]  [ MCP AGENT ]
```
- `[ HARDWARE ]` (`/device`): Hosts `#cardSystemUpdate` at the top, followed by MCU & Silicon, Power & Thermal, and Dual A/B Firmware Slot status.
- `[ STORAGE ]` (`/device/storage`): Hosts `WebUI & Storage (SYSTEM_FS)`, `User Storage (USER_FS)`, and the full-width Flash Partitions bar and geometry table.
- `[ PIN DIAGNOSTICS ]` (`/device/pins`): 30-GPIO matrix.
- `[ REST API ]` (`/device/api`): REST API catalog and tester.
- `[ MCP AGENT ]` (`/device/mcp`): AI model connection config.

*Rationale*: Cleanly isolates storage engineering tools and partition table from high-level silicon health and system update controls.

### 2. Elimination of Recovery Banner in Favor of Unified Update Hero Card
When `SYSTEM_FS` is blank or `lfs_used_bytes === 0`:
- Instead of showing an amber warning `#recoveryBanner`, `#cardSystemUpdate` at the top of the Hardware tab simply displays `WebUI: Not Installed`.
- The 1-click `[ Auto Update ]` button pulls `webui.tar` from GitHub releases and stages it directly.
- The drag-and-drop zone accepts local `.tar` files.

*Rationale*: Eliminates ~60 lines of duplicate DOM elements, duplicate progress bars, and duplicate upload event listeners, saving ~3 KB of in-ROM Flash while presenting a calmer, more professional user experience.

### 3. Multi-Segment All-in-One UF2 Packaging
The packaging script `tools/package_uf2.py` constructs a single UF2 file containing three distinct payload segments with standard RP2350 family ID (`0xe48bff59`):
1. `0x10000000`: Invariant bootloader switcher (`agentla-bootloader.bin`, 8 KB)
2. `0x10004000`: Application firmware runtime (`firmware-rs.bin`, up to 1,024 KB)
3. `0x10204000`: Pre-built LittleFS image (`webui.img`, 512 KB)

*Rationale*: Users dragging a single `.uf2` into RP2350 USB BOOTSEL mode write all three partitions simultaneously, achieving 100% offline out-of-the-box operation with zero extra steps.

### 4. Browser-Orchestrated 2-Phase Auto Update Pipeline
When clicking `[ ⚡ AUTO UPDATE ALL ]`:
1. Client fetches `https://api.github.com/repos/rambros3d/AgentLA/releases/latest`.
2. Phase 1: Downloads `webui.tar` and posts to `/api/v1/system/webui/update`. LittleFS extracts files. Progress updates to 50%.
3. Phase 2: Downloads `agentla-pico2-ota.bin` and posts to `/api/v1/system/firmware/ota`. Inactive slot is written and verified. Progress updates to 100%.
4. Phase 3: Client calls `/api/v1/system/reboot`. Device warm reboots into candidate slot with updated WebUI.

*Rationale*: Keeps firmware `#![no_std]` code minimal and robust while giving users a true 1-click full-system upgrade.

## Risks / Trade-offs

- **[Risk]** Network drop during Phase 2 of 2-Phase update.
  - *Mitigation*: Phase 1 (WebUI) and Phase 2 (Firmware) are staged independently. If Phase 2 fails, the active firmware slot remains untouched, and the device will not reboot into an unverified image.
- **[Risk]** Flash collision between bootloader, firmware, and WebUI in multi-segment UF2.
  - *Mitigation*: Offsets are strictly non-overlapping (`0x10000000`, `0x10004000`, `0x10204000`), matching RP2350 memory-map specifications and unit tests.
