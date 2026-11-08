## 1. CAN WebSocket Bridge Endpoint (`firmware-rs`)

- [x] 1.1 Implement asynchronous CAN WebSocket handler in `firmware-rs/src/rest.rs` supporting JSON frame transmission and reception
- [x] 1.2 Connect WebSocket broadcast to `can_gsusb` / `agentla_bridges` frame callbacks
- [x] 1.3 Register `/api/v1/can/ws` in Picoserve HTTP router in `firmware-rs/src/web.rs`

## 2. Decouple App Seeding from ROM Flash

- [x] 2.1 Remove hardcoded HTML constants (`SERVO_MASTER_INDEX_HTML`, `TERMINAL_INDEX_HTML`) from `firmware-rs/src/apps.rs`
- [x] 2.2 Update `seed_default_apps()` in `firmware-rs/src/apps.rs` to initialize directory trees and manifests for `servo-master`, `terminal`, and `can-sniffer`
- [x] 2.3 Verify `firmware-rs` builds cleanly with strict `#![no_std]` and reduced flash ROM footprint

## 3. CAN Bus Analyzer Web Application (`can-sniffer`)

- [x] 3.1 Create manifest `apps/can-sniffer/app.json` and vector icon `apps/can-sniffer/icon.svg`
- [x] 3.2 Construct `apps/can-sniffer/index.html` with unified Neumorphic header, power rail pill, and safe state controls
- [x] 3.3 Build live frame sniffer table with microsecond timestamps, ID, DLC, hex byte rendering, and ASCII preview
- [x] 3.4 Implement arbitration ID filter, mask matching, and live scroll pausing
- [x] 3.5 Implement single and periodic frame injector with custom ID, DLC, and payload inputs
- [x] 3.6 Wire baud rate selection and $120\,\Omega$ termination resistor toggle via `/api/v1/bridge/can/termination`

## 4. App Storage Directory Synchronization & Packaging

- [x] 4.1 Store official apps (`servo-master`, `terminal`, `can-sniffer`) in `apps/` directory for user-on-demand installation
- [x] 4.2 Remove `webui/apps` to ensure apps are not bundled into default system image and keep `webui.img` lean
- [x] 4.3 Update `webui/apps.html` catalog to offer `servo-master`, `terminal`, and `can-sniffer` as official store apps
- [x] 4.4 Validate `tools/pack_webui.py` builds lean 512 KB system image with 67.2% headroom

## 5. Verification & Test Suite

- [x] 5.1 Add manifest and HTML validation tests in `test/test_can_sniffer_app.py`
- [x] 5.2 Execute test suite to verify all app manifests, REST routes, and build pipelines
