## 1. Build System & Asset Pipeline

- [x] 1.1 Create `tools/gen_board_assets.py` to parse `boards/<board_id>/board.json`, generate `firmware/src/generated/board_config.h`, and stage minified `board.json`, `logo.svg`, and `pinout.svg`.
- [x] 1.2 Update `CMakeLists.txt` to execute `tools/gen_board_assets.py` as a custom command before compilation and dynamically bundle the active board's assets into `pico_set_lwip_httpd_content`.
- [x] 1.3 Add automated schema verification in `tools/gen_board_assets.py` to validate `board.json` against required fields during build.

## 2. Firmware Network & Buffer Hardening

- [x] 2.1 Increase `httpd_post_body` from 768 to 2048 bytes in `firmware/src/network_stack.c` to prevent multi-channel command truncation.
- [x] 2.2 Verify lwIP MIME table mappings for `.json` (`application/json`) and `.svg` (`image/svg+xml`) in `network_stack.c`.

## 3. Client Capability & Feature Registry

- [x] 3.1 Refactor WebUI initialization in `firmware/html/app.js` to fetch `/board.json` on startup with a cache-busting version query string.
- [x] 3.2 Implement `FeatureRegistry` in `firmware/html/app.js` to configure onboard CAN-FD and RS-485 transceivers or switch them to External Breakout Mode, and mount Target Power telemetry widgets when present.
- [x] 3.3 Update top navigation header to render the board's name and dynamically display `/logo.svg`.

## 4. Dynamic Pin Matrix & Partitioning

- [x] 4.1 Replace hardcoded pin arrays in `firmware/html/app.js` with a dynamic generator reading `board.zones` and `gpio_range`.
- [x] 4.2 Render fixed hardware function groups (Programmer, Transceivers, Scope) with connector pins and level-shift tags (`1.2V - 5.0V`), alongside a Unified Flexible Digital Pool (PIO, logic analyzer, signal generator).
- [x] 4.3 Implement dynamic pin state and conflict detection across all declared GPIOs (up to 48 GPIOs for BakuProbe).

## 5. Interactive Pinout SVG Integration

- [x] 5.1 Implement `mountInteractivePinout()` in `firmware/html/app.js` to asynchronously fetch `/pinout.svg` and inline the vector markup into `#hardware-pinout-canvas` inside the Pin Matrix view (`#view-pin-matrix`).
- [x] 5.2 Attach pointer event listeners to `.board-pin` elements to provide bidirectional hover highlighting between SVG pins and the Pin Matrix table.
- [x] 5.3 Implement pin click popover displaying the pin's active instrument, alternate roles, and electrical parameters.

## 6. Verification & Automated Testing

- [x] 6.1 Test build against `pico2` profile and verify generated headers and static web assets.
- [x] 6.2 Test build against `bakuprobe` profile and verify 48-GPIO zones, transceiver flags, and static web assets.
- [x] 6.3 Add a Python test in `test/test_board_manifests.py` verifying schema conformance and asset presence for all board bundles.
