## 1. Unified Neumorphic Depth System (Dark & Light)

- [x] 1.1 In `firmware/html/style.css`, update Dark Mode ("Forest") luminance tokens: set `--color-base-100` (page canvas) to `oklch(18% 0 0)` and `--color-base-200` (card surface) to `oklch(24% 0 0)` to restore elevated card hierarchy.
- [x] 1.2 In `firmware/html/style.css`, implement the shared dual-shadow neumorphic formula: `--card-neumorphic-shadow` combining ambient drop shadow (`0 6px 18px rgba(0, 0, 0, 0.65)` in dark, `0 4px 14px rgba(0, 0, 0, 0.08)` in light) and top-left specular rim catch highlight (`inset 0 1px 0 rgba(255, 255, 255, ...)`).
- [x] 1.3 In `firmware/html/style.css`, soften `--border-color` in dark mode to translucent `rgba(255, 255, 255, 0.08)` to replace harsh wireframe outlines with soft surface creases.
- [x] 1.4 Apply `--card-neumorphic-shadow` across all major panels (`.station-card`, `.adapter-card`, `.bento-card`, `.domain-card`, `.pinout-card`).

## 2. Top-Bar Target Power Pill

- [x] 2.1 In `firmware/html/index.html`, add the Target Power pill button (`#btn-target-power-pill`) immediately to the left of `#btn-emergency-safe` in `#top-bar`.
- [x] 2.2 Style `#btn-target-power-pill` in `firmware/html/style.css` with active state (emerald tint, pulsing green dot, bolt icon, live text) and standby state (muted dot, `POWER OFF`).
- [x] 2.3 In `firmware/html/app.js`, wire up `#btn-target-power-pill` click handler to dispatch `POWER ON` or `POWER OFF` over `/mcp` or `/api/power`.
- [x] 2.4 In `firmware/html/app.js`, integrate live voltage and INA180 current telemetry updates into the watchdog heartbeat loop, updating `#top-bar-voltage-val` and `#top-bar-current-val`.

## 3. Streamlined Two-Column Debugger View

- [x] 3.1 In `firmware/html/index.html`, remove the dedicated `VTarget Power Rail Card` from `#view-target-station`.
- [x] 3.2 In `firmware/html/index.html` and `style.css`, restructure `#view-target-station` into a clean two-column layout: Universal Target Flasher on the left (~400px), Target SWD / CMSIS-DAP Debugger & Memory Viewer on the right (`flex: 1`).
- [x] 3.3 Ensure drag-and-drop flashing, chip architecture selectors, execution controls (`Halt`, `Resume`, `Step`, `Reset`), core registers grid, and memory viewer bind correctly in `firmware/html/app.js`.

## 4. Master-Detail USB Protocol Adapters View

- [x] 4.1 In `firmware/html/index.html`, replace the 6-card bento grid in `#view-adapters` with a Master-Detail layout container (`.adapters-master-detail`).
- [x] 4.2 In `firmware/html/index.html`, construct the left master list (`.adapters-master-list`) displaying live status dots `●`, protocol names, active baud/clock rates, and hardware pins for UART, I2C, I3C, SPI, CAN, and Industrial bridges.
- [x] 4.3 In `firmware/html/index.html`, construct the right detail workspace (`.adapters-detail-pane`) with persistent log containers for each protocol, transaction inputs, action buttons, and quick preset chips (`Scan Bus`, `Read 24C02 EEPROM`, `Ping OBD-II`).
- [x] 4.4 In `firmware/html/style.css`, style `.adapters-master-detail` for desktop ($\ge 900\text{px}$) as side-by-side split, and on mobile ($< 900\text{px}$) as top scrollable pill bar with detail pane below.
- [x] 4.5 In `firmware/html/app.js`, implement protocol switcher logic that reveals the selected detail pane while preserving all background DOM log containers, defaulting to UART or the active hardware profile's claimed adapter.
- [x] 4.6 Wire quick preset chips to auto-fill transaction inputs and dispatch commands.

## 5. Streamlined Active Bench & Collapsible Signal Generator Drawer (Deferred - Pending Further Refinement)

<!-- Active Bench controls and Signal Generator drawer deferred per user request for further UX refinement -->

## 6. Verification & Firmware Staging

- [x] 6.1 Re-stage web assets and rebuild firmware with `make -C build-work -j4 AgentLA`.
- [x] 6.2 Run test suite: `pytest test/test_board_manifests.py test/test_protocol_decoders.py test/test_sigrok_export.py -v`.
- [x] 6.3 Validate responsive behavior across desktop ($\ge 900\text{px}$) and mobile ($< 900\text{px}$).
