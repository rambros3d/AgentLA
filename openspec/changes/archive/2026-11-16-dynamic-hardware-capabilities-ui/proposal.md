## Why

The AgentLA WebUI and backend telemetry engine were designed around the flagship BakuProbe and Demo hardware profiles, causing UI surfaces and firmware telemetry to assume full hardware capabilities even when running on a baseline Raspberry Pi Pico 2. On a Pico 2, which lacks target power switches, current sense ICs, differential transceivers, and level translators:
- The WebUI top bar unconditionally displays a target power toggle switch and simulated or bogus voltage/current numbers (`3.31V · 14mA`), implying switchable power when none exists.
- The firmware telemetry engine reads RP2350 internal ADC 1 and ADC 2 as current and voltage, interpreting user header pins GP27 and GP28 as current shunt measurements.
- The App Store catalog and pin matrix claim support for level shifting (`1.2V-5.0V`) and hardware transceivers (`CAN`, `RS-485`) regardless of the board's manifest.

This change aligns the WebUI, App Store, and firmware telemetry engine to dynamically adapt their interface, controls, and diagnostics to the active board manifest (`board.json`).

## What Changes

- **WebUI Top-Bar Power Widget Adaptation**:
  - Dynamically hides or disables the power toggle switch when `target_power_control` is `false`.
  - Replaces the toggle switch with an informational rail badge (e.g. `3.3V Fixed (Always On)`) when target power is non-switchable.
  - Hides current draw in mA and power in mW when `target_current_sense` is `false`.
- **Firmware Telemetry Guarding**:
  - In `firmware-rs/src/telemetry.rs`, suppresses sampling of ADC channels for current sense when `target_current_sense` is disabled in the active board profile, preventing floating noise on GP27 from being reported as target current or triggering overcurrent High-Z safety trips.
  - Reports `null` or explicit `unsupported` status for target current and power metrics when current sensing is absent.
- **Dynamic Pin Matrix & Level Shifting Presentation**:
  - Updates `app.js` and `generatePinMapFromManifest()` to check `manifest.level_shifting` rather than hardcoding `1.2V–5.0V Level-Shifted` on programmer pins for direct 3.3V boards like Pico 2.
  - Leaves flexible pool pins unencumbered by hardcoded `RS-485` or `CAN` labels when `has_hardware_rs485` and `has_hardware_can` are `false`.
- **App Store Hardware Compatibility Validation**:
  - Updates `webui/index.html` application sheet and URL installer to compare `app.requires` against `BOARD_MANIFEST.features`, displaying an explicit hardware missing warning if required peripherals (such as `has_hardware_can`) are not present on the active board.

## Capabilities

### New Capabilities
- `hardware-capability-adaptation`: Dynamic client-side and server-side feature validation ensuring that UI controls, App Store compatibility checks, and telemetry endpoints strictly reflect board profile capabilities (`board.json`).

### Modified Capabilities
- `webui-control-surface`: Top-bar power controls, zone badges, and pin roles must dynamically reflect board profile capabilities.
- `rest-device-telemetry`: Telemetry sampling and streaming must respect board current-sensing and power-control capabilities.

## Impact

- `webui/app.js`, `webui/agentla-bridge.js`, `webui/index.html`, `webui/terminal.html`, `webui/lab.html`, `webui/barefoc.html`: Top-bar power widget, pin matrix generator, and App Store compatibility checker.
- `firmware-rs/src/telemetry.rs`: Hardware ADC sampling and JSON telemetry formatter.
- APIs: `/api/v1/system/telemetry` and `/api/v1/system/telemetry/stream` payloads safely handle boards lacking current sense.
