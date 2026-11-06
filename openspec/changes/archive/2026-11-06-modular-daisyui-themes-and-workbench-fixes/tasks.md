## 1. Theme Definitions & Build Tooling

- [x] 1.1 Create `themes/emerald.css`, `themes/forest.css`, and `themes/caramellatte.css` using raw DaisyUI `@plugin "daisyui/theme"` format
- [x] 1.2 Implement `tools/compile_themes.py` to parse theme files, generate `firmware/html/themes.css`, and output `firmware/html/themes.json`
- [x] 1.3 Hook theme compiler into CMake build system and `tools/gen_board_assets.py`

## 2. Firmware REST API & SSI Integration

- [x] 2.1 Implement `/api/v1/settings` (`GET` & `POST`) and `/api/v1/themes` (`GET` & `POST`) in `firmware/src/rest_api.c` backed by LittleFS flash
- [x] 2.2 Wire lwIP SSI tag `<!--#THEME-->` in `firmware/src/network_stack.c` and update `<html lang="en" data-theme="<!--#THEME-->">` in `index.html`

## 3. WebUI Theme Engine & Tri-State Navigation Switcher

- [x] 3.1 Implement tri-state theme switcher in top navigation bar (`system` &rarr; `light` &rarr; `dark` &rarr; `system`) with Solar BoldDuotone icons (`sun`, `moon`, and split `sun-fog`)
- [x] 3.2 Add Default Light and Default Dark theme dropdown selectors to Settings Modal (`#modal-settings`)
- [x] 3.3 Implement "Add Theme" modal with `@plugin` CSS paste area, live test preview, and save to `/api/v1/themes`
- [x] 3.4 Purge obsolete hue slider and replace hardcoded `var(--accent-cyan)` tokens with semantic theme tokens
- [x] 3.5 Isolate high-frequency MSO splitter dragging to browser `localStorage` (`agentla_mso_split`)

## 4. Scope Tab Fixes & Virtual Channel Mapping

- [x] 4.1 Fix CSS class mismatches in Pin Config Modal (`.color-swatch-btn` / `.swatch-item`) and Scope Pins Bar (`.configured-digital` / `.active-digital`)
- [x] 4.2 Implement dynamic virtual channel multiplexing in `app.js` mapping active probe pins `GP10`–`GP29` to logic analyzer lanes `D0`–`D7`

## 5. Adapter Tab & Pin Matrix UI Fixes

- [x] 5.1 Route bus master transmissions (UART, I2C, SPI, CAN, Modbus) to dedicated protocol log windows via `appendAdapterLog()`
- [x] 5.2 Style Pin Matrix instant GPIO action buttons (`.btn-pin-act` for `H`, `L`, `PU`, `PD`, `Z`) with DaisyUI button groups
- [x] 5.3 Implement dynamic (icon - icon/text) sidebar navigation tabs in `.desktop-vertical-nav` matching bottom nav behavior

## 6. Verification & Firmware Build

- [x] 6.1 Validate JavaScript syntax and mock bridge (`node -c firmware/html/app.js web/demo/mock-bridge.js`)
- [x] 6.2 Execute Python automated test suite (`pytest test/`)
- [x] 6.3 Verify clean firmware compilation (`make -C build-work -j4 AgentLA`)
