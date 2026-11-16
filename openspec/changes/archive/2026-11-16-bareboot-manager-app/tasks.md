## 1. Firmware Raw Bus Transfer Endpoints

- [x] 1.1 Implement raw I2C stream transfer handler in `firmware-rs/src/rest.rs`
- [x] 1.2 Implement raw I3C SDR transfer handler in `firmware-rs/src/rest.rs`
- [x] 1.3 Register `/api/v1/bus/i2c/transfer` and `/api/v1/bus/i3c/transfer` routes in `firmware-rs/src/web.rs`

## 2. App Manifest & Target Firmware Catalog

- [x] 2.1 Create `apps/bareboot/` directory and define `app.json` manifest
- [x] 2.2 Create industrial SVG icon in `apps/bareboot/icon.svg` without emojis
- [x] 2.3 Create `apps/bareboot/catalog.json` defining target MCU profiles, memory boundaries, and transports
- [x] 2.4 Bundle pre-compiled BareBoot binaries under `apps/bareboot/firmwares/`

## 3. Client-Side Katapult Protocol Engine

- [x] 3.1 Implement Katapult frame encoder and CCITT CRC16 calculation in JavaScript
- [x] 3.2 Implement sliding-window frame scanner and trailer validator
- [x] 3.3 Implement transport abstractions for UART WebSocket, CAN WebSocket, and I2C/I3C REST transfers
- [x] 3.4 Implement handsfree entry triggers (UART token, I2C/I3C byte `0xF0`, CAN reboot frame `0x3F0`, and nRST toggle)
- [x] 3.5 Implement session state machine for `CONNECT`, block streaming (`RX_BLOCK`), `RX_EOF`, and `COMPLETE`

## 4. UI & Workbench Layout

- [x] 4.1 Construct `apps/bareboot/index.html` layout utilizing DaisyUI, Tailwind, and `agentla-bridge.js`
- [x] 4.2 Implement Tab 1 (Bootloader Flasher) with target detection and SWD flash controls
- [x] 4.3 Implement Tab 2 (Configuration Editor) with interactive `bareboot.toml` generator
- [x] 4.4 Implement Tab 3 (Firmware Updater) with bus selection, live progress bar, throughput metrics, and packet log

## 5. App Packaging & Verification

- [x] 5.1 Package `apps/packages/bareboot.tar` and update `apps/catalog.json`
- [x] 5.2 Verify AgentLA design compliance (no CSS dot indicators, bottom-anchored dock, full-bleed viewport, zero emojis)
- [x] 5.3 Verify protocol framing and transfer flow with test fixtures
