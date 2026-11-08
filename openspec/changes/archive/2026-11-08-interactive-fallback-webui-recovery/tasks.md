## 1. Interactive Fallback UI Asset

- [x] 1.1 Create `firmware-rs/assets/recovery.html` matching the Neo-Industrial design language (`--bg:#09090b; --panel:#111114`) with self-contained styles and zero external dependencies
- [x] 1.2 Implement in-page `webui.tar` file selector and drag-and-drop zone with streaming upload to `POST /api/v1/system/webui/update` and progress bar
- [x] 1.3 Implement 1-click GitHub update button querying GitHub Releases API from client JavaScript to pull and flash `webui.tar`
- [x] 1.4 Add prominent `[ Open Diagnostics Console (/device) ]` navigation button linking to `/device`

## 2. Firmware Integration

- [x] 2.1 Integrate `recovery.html` into `firmware-rs/src/fs_http.rs` via `include_str!` to serve the interactive recovery interface when `SYSTEM_FS` lacks requested templates
- [x] 2.2 Verify firmware compilation and `#![no_std]` zero-heap compliance using `cargo check --target thumbv8m.main-none-eabihf`

## 3. Automated Testing & Live Verification

- [x] 3.1 Add pytest test cases in `test/test_device_diagnostics.py` verifying `recovery.html` structure, zero external CDN links, upload endpoints, and `/device` navigation link
- [x] 3.2 Build release binary and flash to Raspberry Pi Pico 2 via Debugprobe (`probe-rs`)
- [x] 3.3 Verify live response on `http://10.10.10.10/home` and `/device` navigation
