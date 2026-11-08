## 1. Firmware LittleFS Settings Persistence

- [x] 1.1 Implement LittleFS loading of `/system/settings.json` during boot in `firmware-rs/src/rest.rs` (populating `UI_SETTINGS`).
- [x] 1.2 Implement LittleFS saving to `/system/settings.json` on `POST` and `PUT` to `/api/v1/settings` in `firmware-rs/src/rest.rs`.
- [x] 1.3 Ensure default fallback for `default_home_route` is `"/home"` in `get_default_home_route()`.

## 2. Firmware Routing Streamlining

- [x] 2.1 Update `firmware-rs/src/web.rs` route definitions to register canonical routes: `/home`, `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, and `/terminal`.
- [x] 2.2 Remove deprecated legacy routes (`/lab`, `/apps`, `/target`, `/console`, `/pin`) from `firmware-rs/src/web.rs`.

## 3. WebUI & Simulator Canonical Route Alignment

- [x] 3.1 Update `webui/app.js` default home route from legacy `'/lab'` to `'/home'` and map canonical routes (`/pins`, `/agent`, `/debug`, `/adapter`, `/scope`).
- [x] 3.2 Update `webui/apps.html` default home route from legacy `'/lab'` to `'/home'` and harmonize landing page dropdown selector.
- [x] 3.3 Update `webui/index.html` settings modal dropdown options to strictly list canonical routes: `/home`, `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, `/terminal`.
- [x] 3.4 Update `webui/mock-bridge.js` to default to `/home` and simulate persistent settings via `localStorage`.

## 4. Verification & Automated Tests

- [x] 4.1 Update unit tests in `test/test_webui_assets.py` to verify `/home` default landing setting and canonical route options.
- [x] 4.2 Verify clean firmware compilation (`cargo check --manifest-path firmware-rs/Cargo.toml --target thumbv8m.main-none-eabihf`) and test suite execution.
