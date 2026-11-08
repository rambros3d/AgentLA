## 1. Asset Restructuring & Internal Links

- [x] 1.1 Rename `webui/index.html` to `webui/lab.html` and `webui/apps.html` to `webui/index.html`.
- [x] 1.2 Update navigation links, header anchors, and settings modal in `webui/index.html` and `webui/lab.html`.
- [x] 1.3 Update script cross-references in `webui/terminal.html`, `webui/app.js`, and `webui/mock-bridge.js`.

## 2. Firmware Route Dispatch Updates

- [x] 2.1 Update `firmware-rs/src/web.rs` routing: serve `index.html` for `/home`, `/home/`, and `/index.html`.
- [x] 2.2 Update `firmware-rs/src/web.rs` routing: serve `lab.html` for `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, and `/lab.html`.
- [x] 2.3 Verify `/terminal`, `/terminal/`, and `/terminal.html` serve `terminal.html`.

## 3. Website Portal & Build Script Alignment

- [x] 3.1 Update `website/index.html` iframe preview source to `demo/lab.html?embed=1`.
- [x] 3.2 Verify `tools/build_portal.py` and `tools/pack_webui.py` handle `index.html`, `lab.html`, and `terminal.html`.

## 4. Verification & Testing

- [x] 4.1 Update and run `test/test_webui_assets.py`.
- [x] 4.2 Run `cargo check --manifest-path firmware-rs/Cargo.toml --target thumbv8m.main-none-eabihf`.
- [x] 4.3 Run `pytest test/test_webui_assets.py test/test_multi_app_store.py test/test_board_manifests.py`.
- [x] 4.4 Build static portal with `python3 tools/build_portal.py --output _site`.
