## 1. Embedded HTML Asset & Client Logic

- [x] 1.1 Create self-contained `firmware-rs/assets/device.html` (< 6 KB) with inline dark-mode CSS styles and responsive card layout
- [x] 1.2 Implement client JavaScript in `device.html` to fetch and render `/api/board/manifest`, `/api/v1/system/telemetry`, and `/api/v1/system/firmware/status`
- [x] 1.3 Implement browser-mediated GitHub release checker comparing installed versions with remote assets and handling Auto Updates and local file uploads
- [x] 1.4 Implement interactive Developer Mode controls in `device.html` for BOOTSEL reboot, slot rollback/commit, pin release/toggle, and partition format

## 2. Firmware REST Endpoints & Routing

- [x] 2.1 Register `/device` and `/device.html` in `firmware-rs/src/web.rs` pointing to the ROM-embedded asset via `File::html`
- [x] 2.2 Add `POST /api/v1/system/bootsel` endpoint to trigger `reset_to_usb_boot(0, 0)`
- [x] 2.3 Add `POST /api/v1/system/reboot` endpoint to trigger hardware watchdog reset
- [x] 2.4 Add `POST /api/v1/system/pins/release_all` endpoint to emergency release all claimed pins in `pin_owner`
- [x] 2.5 Add `POST /api/v1/system/pins/toggle` endpoint to temporarily drive or probe a free pin
- [x] 2.6 Add `POST /api/v1/system/fs/format` endpoint to format `SYSTEM_FS` or `USER_FS` partitions

## 3. Build, Verification & Testing

- [x] 3.1 Verify firmware compilation with `cargo check` and `cargo build --release` targeting `thumbv8m.main-none-eabihf`
- [x] 3.2 Add automated unit and routing tests in `test/test_device_diagnostics.py`
- [x] 3.3 Flash firmware to the connected Raspberry Pi Pico 2 board via Debugprobe and verify live HTTP responses for `/device` and `/device.html`
- [x] 3.4 Verify developer actions on hardware (telemetry rendering, pin release, and update checks)
