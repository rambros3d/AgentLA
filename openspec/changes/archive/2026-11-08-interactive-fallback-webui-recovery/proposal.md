## Why

When Partition 1 (`SYSTEM_FS`) is unpopulated, formatted, or corrupted, navigating to root `/` or system web routes (`/home`, `/scope`, etc.) serves a static, plain-text fallback page that instructs users to flash `webui.uf2` or upload a tarball over curl, but provides no interactive controls or direct recovery path. Users are stranded unless they manually type `http://10.10.10.10/device` or resort to command-line curl tools.

## What Changes

- Replace the plain-text `RECOVERY_HTML` with a self-contained, interactive Neo-Industrial WebUI recovery page served directly from ROM.
- Provide a direct file picker and drag-and-drop zone to upload `webui.tar` directly into `POST /api/v1/system/webui/update` with live streaming progress and automatic page reload.
- Provide a 1-click "Download & Install Latest from GitHub" button that pulls `webui.tar` via client-side GitHub Releases API and pipes it to the probe flasher.
- Add a prominent, high-contrast `[ Open Diagnostics Console (/device) ]` button to instantly switch to the full In-ROM device diagnostics interface.
- Keep the entire fallback page in ROM with zero heap allocation and minimal flash footprint (~1.5 KB).

## Capabilities

### New Capabilities
- `interactive-fallback-recovery`: Self-contained In-ROM interactive WebUI recovery interface offering direct tarball flashing, GitHub release pulling, and instant navigation to `/device`.

### Modified Capabilities
- `rom-device-diagnostics`: Interconnection and navigation flow between the unpopulated partition fallback interface and the `/device` console.
- `dual-littlefs-partitions`: Unpopulated partition recovery semantics for `SYSTEM_FS`.

## Impact

- `firmware-rs/src/fs_http.rs`: Update `RECOVERY_HTML` to the interactive recovery interface or move to `firmware-rs/assets/recovery.html` via `include_str!`.
- No impact on SRAM / zero-heap `#![no_std]` footprint.
- Adds ~1.5 KB to the firmware binary in Slot A (where >640 KB is free).
