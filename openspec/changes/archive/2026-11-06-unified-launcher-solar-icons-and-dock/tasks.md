## 1. Top Bar Unification & Hardware Profile Dropdown

- [x] 1.1 Update `firmware/html/apps.html` top bar markup to include `#profile-dropdown-wrapper` and match `firmware/html/index.html`.
- [x] 1.2 Wire up hardware profile dropdown JavaScript handlers (`/api/profiles`, active profile selection, click-outside listener) in `apps.html`.
- [x] 1.3 Ensure the AgentLA logo `#brand-logo` in `apps.html` is display-only with default cursor and no navigation action.

## 2. Core Apps Section & Solar Icons

- [x] 2.1 Rename section header from "CORE INSTRUMENTS & FLAGSHIP" to "Core Apps" with Solar section icon in `apps.html`.
- [x] 2.2 Replace placeholder SVGs with authentic 2-tone Solar Icons for AgentLA Lab (flagship), MSO Scope, Pinout Matrix, USB Adapters, and Console in `apps.html`.
- [x] 2.3 Update squircle tile styling and hover effects in `firmware/html/style.css` to match Lab `.dash-tile` conventions.

## 3. User Installed App Custom Icon Resolution Pipeline

- [x] 3.1 Update `renderInstalledApps()` in `apps.html` to evaluate `app.iconSvg` (inline SVG), then `app.icon` / root `icon.svg` / `icon.png` (`<img class="os-app-custom-img">`), with an `onerror` fallback.
- [x] 3.2 Provide an authentic 2-tone Solar application box icon as the default fallback for apps without custom graphics.
- [x] 3.3 Add `.os-app-custom-img` styling in `firmware/html/style.css` for clean 28×28px rendering inside squircle icon containers.

## 4. OS Bottom Dock for Storage Telemetry

- [x] 4.1 Remove in-flow storage card from `apps.html` body and implement fixed bottom DaisyUI `.dock` (`#storage-dock`).
- [x] 4.2 Populate the dock with storage icon, segmented progress bar (ROM, Apps, Free), and live formatted capacity text.
- [x] 4.3 Add bottom safe-area padding (`padding-bottom: 5rem`) to `.launcher-workspace` and `.launcher-container` in `firmware/html/style.css`.
- [x] 4.4 Update `updateStorageDisplay()` in `apps.html` to drive the dock's segmented bar and text metrics.

## 5. Verification & Parity Sync

- [x] 5.1 Sync changes to `web/apps.html` for static marketing/preview parity.
- [x] 5.2 Run board asset generator: `python3 tools/gen_board_assets.py --board pico2 --base-html-dir firmware/html --output-web-dir firmware/html/generated`.
- [x] 5.3 Execute test suite: `pytest test/test_webui_assets.py test/test_multi_app_store.py`.
- [x] 5.4 Build firmware: `make -C build-work -j4 AgentLA` and verify clean compilation with 0 KB flash bloat.
