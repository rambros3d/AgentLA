## Why

When the LittleFS partitions (`SYSTEM_FS` / `USER_FS`) are unformatted, corrupt, or undergoing recovery, the standard WebUI applications (`/home`, `/scope`, etc.) cannot load. Users and firmware developers need an infallible, zero-dependency diagnostics and rescue console permanently embedded in the microcontroller's Flash ROM. This page must report detailed hardware health, provide an in-browser update checker with individual firmware/WebUI flasher capabilities, and offer essential developer-mode recovery tools (hands-free BOOTSEL reboot, slot controls, pin arbiter overrides, and partition formatting).

## What Changes

- Add a standalone, self-contained HTML page (`device.html`) embedded directly in Flash ROM via `include_str!` / `File::html`.
- Register canonical routes `/device` and `/device.html` in the picoserve routing engine.
- Implement live hardware inspection on `/device`: MCU specifications, core clock, SRAM usage, LittleFS partition status, live voltages, currents, temperatures, and full 30-GPIO pin ownership matrix.
- Implement a browser-mediated update checker that queries latest GitHub releases, displays changelogs/versions, and provides independent Auto Update buttons (plus local file upload fallbacks) for:
  - Firmware (`.bin`) via `POST /api/v1/system/firmware/ota`
  - WebUI (`.tar`) via `POST /api/v1/system/webui/update`
- Provide always-visible Developer Mode cards:
  - **Flash & Recovery**: Hands-free reboot to BOOTSEL (UF2 drive mode via `reset_to_usb_boot`), soft watchdog reboot, A/B slot flip and rollback controls, and format buttons for `SYSTEM_FS` and `USER_FS`.
  - **Pin Arbiter Overrides**: Emergency release of all pin claims, and manual pin toggle / continuity probe (High / Low / Hi-Z).
- Add supporting REST API endpoints:
  - `POST /api/v1/system/bootsel`
  - `POST /api/v1/system/reboot`
  - `POST /api/v1/system/pins/release_all`
  - `POST /api/v1/system/pins/toggle`
  - `POST /api/v1/system/fs/format`

## Capabilities

### New Capabilities
- `rom-device-diagnostics`: Standalone, ROM-embedded hardware diagnostics, rescue console, and developer controls on `/device` and `device.html`.

### Modified Capabilities
- `url-routing-engine`: Add canonical routing for `/device` and `/device.html` pointing to the ROM-embedded file.
- `probe-firmware-ota`: Support developer recovery actions including soft BOOTSEL entry and individual firmware vs WebUI component updates.

## Impact

- **Firmware ROM**: ~5–6 KB Flash footprint for `device.html` and supporting picoserve route handlers.
- **Microcontroller Driver**: Adds `reset_to_usb_boot` rom-call hook on RP2350 and watchdog reboot trigger.
- **REST Endpoints**: Adds `/api/v1/system/bootsel`, `/api/v1/system/reboot`, `/api/v1/system/pins/release_all`, `/api/v1/system/pins/toggle`, `/api/v1/system/fs/format`.
- **Zero LittleFS Dependency**: The `/device` page functions 100% offline and intact even if LittleFS is completely unformatted.
