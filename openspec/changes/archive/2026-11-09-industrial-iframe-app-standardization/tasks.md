## 1. System Applications Layout & Industrial Updates

- [x] 1.1 Invert `webui/terminal.html` layout with top-anchored input bar, reverse-chronological stream rendering, sub-toolbar, and 72px dock margin
- [x] 1.2 Update `webui/files.html` to add `id="top-bar"`, relocate partition switcher to sub-toolbar, and add 72px dock margin

## 2. User Applications Layout & Industrial Updates

- [x] 2.1 Update `apps/canbus/index.html` with fixed top Bento TX Frame Injector, industrial styling, and 72px dock margin
- [x] 2.2 Update `apps/neopixel/index.html` with dedicated in-app sub-toolbar for shaders/blackout/telemetry, industrial styling, and 72px dock margin
- [x] 2.3 Update `apps/servo/index.html` with in-app sub-toolbar for calibration/gamepad, industrial Bento cards, and 72px dock margin

## 3. Catalog Packaging & Build Verification

- [x] 3.1 Rebuild user app tar packages with `python3 tools/build_catalog.py`
- [x] 3.2 Pack system WebUI with `python3 tools/pack_webui.py`
- [x] 3.3 Verify LittleFS flash footprint and test suite (`pytest test/`)
