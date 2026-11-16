## 1. FeatureRegistry & Top-Bar Power Widget Adaptation

- [x] 1.1 Extend `FeatureRegistry` in `webui/app.js` and `webui/agentla-bridge.js` to reconcile `#btn-target-power-pill`, `#target-power-switch`, and `#top-bar-power-text` with `target_power_control` and `target_current_sense` from `board.json`.
- [x] 1.2 Implement graceful state degradation for `#btn-target-power-pill`: hide/disable the toggle switch and display a static rail badge (e.g. `3.3V Fixed Rail`) when `target_power_control` is false.
- [x] 1.3 Suppress target current in mA and power in mW from top-bar readouts and telemetry popovers when `target_current_sense` is false.

## 2. Dynamic Pin Matrix & Zone Translation Accuracy

- [x] 2.1 Update `generatePinMapFromManifest()` in `webui/app.js` to evaluate `manifest.level_shifting` rather than hardcoding `1.2V–5.0V Level-Shifted` tags for direct 3.3V boards.
- [x] 2.2 Ensure flexible pool pins on boards without hardware transceivers (e.g. Raspberry Pi Pico 2) are labeled as general GPIOs rather than fixed CAN or RS-485 transceiver roles.

## 3. Firmware Telemetry Safety & Gating

- [x] 3.1 Update `firmware-rs/src/telemetry.rs` to gate INA180 ADC1 current sampling and overcurrent High-Z tripping against active board profile capabilities.
- [x] 3.2 Ensure telemetry JSON serialization and SSE streams report safe fallback values (`0` or `null`) for `i_target_ma` when `target_current_sense` is false.

## 4. App Store Hardware Requirements Validation

- [x] 4.1 Update `webui/index.html` application detail sheet to cross-reference `app.requires` with `BOARD_MANIFEST.features`.
- [x] 4.2 Display an explicit hardware incompatibility warning in the App Store sheet when required peripherals (e.g., `has_hardware_can`) are missing from the active board.

## 5. Verification & Testing

- [x] 5.1 Verify WebUI rendering against `boards/pico2/board.json` to confirm power toggle removal, accurate 3.3V pin labeling, and CAN app hardware requirement warning.
- [x] 5.2 Verify WebUI rendering against `boards/bakuprobe/board.json` to confirm that full power switching, INA180 telemetry, and transceiver badges remain functional.
