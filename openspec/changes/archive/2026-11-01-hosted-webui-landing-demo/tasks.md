## 1. Directory Structure & Firmware Verification

- [x] 1.1 Create `web/` and `web/demo/` standalone directory hierarchy
- [x] 1.2 Verify `CMakeLists.txt` and `firmware/html/` remain cleanly separated with 0 overhead in `pico_fsdata.inc`

## 2. Marketing & Discovery Splash Page

- [x] 2.1 Build `web/index.html` landing page with product hero banner, hardware feature grid, and live interactive pinout summary
- [x] 2.2 Implement local subnet probe discovery (`http://192.168.7.1` ping check and 1-click launch link)
- [x] 2.3 Style landing page in `web/landing.css` with responsive layout, high-contrast dark palette, and mobile tap targets

## 3. In-Browser Demo WebUI & Mock Bridge

- [x] 3.1 Adapt WebUI assets for `web/demo/` with a dedicated demo mode indicator and simulator hooks
- [x] 3.2 Build `web/demo/mock-bridge.js` providing synthetic 100 MS/s logic captures (I2C, SPI, UART) and simulated telemetry updates
- [x] 3.3 Implement simulated 30-pin dynamic matrix playground enforcing GPIO 0–9 fixed programmer reservation and GPIO 10–29 dynamic claims
- [x] 3.4 Implement WebSerial USB connection helper in `web/demo/mock-bridge.js` for supported browsers

## 4. Deployment Workflow & Validation

- [x] 4.1 Create GitHub Actions workflow `.github/workflows/deploy-pages.yml` to publish `web/` to GitHub Pages
- [x] 4.2 Verify static portal loading and simulator interactions locally
- [x] 4.3 Recompile firmware (`make -C build-work AgentLA`) to guarantee firmware binary remains unaffected
