## Context

AgentLA supports multiple board targets:
1. `bakuprobe`: RP2350B, 48 GPIOs, dedicated power switch (P-FET via 74HC595 bit 24), dedicated 4-channel SPI ADC (TI ADC124S021), TI INA180 dynamic current shunt, dual-supply level translators (1.65V–5.5V), onboard SIT1044 CAN-FD and SN75176 RS-485 transceivers.
2. `pico2`: RP2350A, 30 GPIOs, unswitched VBUS / 3.3V rail, direct 3.3V SIO logic, no current shunt or power ADC, 4 direct RP2350 ADC lines (GP26–GP29), no onboard differential transceivers.
3. `demo`: Flagship simulator reference profile.

Previously, `webui/index.html`, `webui/app.js`, `webui/agentla-bridge.js`, and `firmware-rs/src/telemetry.rs` made hardcoded assumptions:
- The top-bar target power widget was hardcoded into the HTML markup of every page with a toggle switch and `3.31V · 14mA` label.
- The telemetry engine in `firmware-rs/src/telemetry.rs` sampled RP2350 ADC 1 as current and ADC 2 as voltage without checking whether the board actually features an INA180 sensor.
- The pin matrix stamped fixed `1.2V–5.0V Level-Shifted` tags onto programmer pins and hardcoded CAN/RS485 pin labels even on flexible pool pins.
- The App Store detail modal unconditionally reported `[OK] Requires hardware module` without validating `BOARD_MANIFEST.features`.

## Goals / Non-Goals

**Goals:**
- Provide a unified, reactive capability-adaptation mechanism in `FeatureRegistry` that modifies WebUI components (power pill, pin matrix, badges, app store) based on `board.json`.
- When `target_power_control` is `false`: hide or disable the toggle switch, displaying an informative static badge (e.g. `3.3V Direct Rail`) without interactive power toggle actions.
- When `target_current_sense` is `false`: hide mA and mW readouts from the top bar and telemetry HUD.
- In `firmware-rs/src/telemetry.rs`: guard target current calculations against board capabilities. When current sensing is unsupported, output `null` or 0 for `i_target_ma` and `p_target_mw`, and disable overcurrent High-Z tripping on floating pins.
- Correct `generatePinMapFromManifest()` to check `manifest.level_shifting` rather than hardcoding `1.2V–5.0V` across all boards.
- Validate `app.requires` against `BOARD_MANIFEST.features` in the App Store sheet, presenting a clear hardware incompatibility badge when required modules (e.g., `has_hardware_can`) are not present.

**Non-Goals:**
- Synthesizing virtual software power switches or emulation on boards lacking hardware switches.
- Changing physical pin allocations of existing boards.
- Modifying LittleFS application package formats.

## Decisions

### 1. Centralized Feature Gating via `FeatureRegistry`
- **Choice**: Extend `FeatureRegistry` in `webui/app.js` and `webui/agentla-bridge.js` to serve as the single source of truth for UI adaptation.
- **Rationale**: Currently, `FeatureRegistry` only manages transceiver mode badges and one drawer element. Expanding `FeatureRegistry.applyAll()` ensures all DOM elements (`#btn-target-power-pill`, `#target-power-switch`, `#top-bar-power-text`, `#card-adapter-*`, `#matrix-domains-container`) are reconciled immediately whenever `board.json` loads.

### 2. Top-Bar Power Pill State Degradation Matrix
- **Choice**: Implement three distinct UI presentations for `#btn-target-power-pill`:
  - `Full` (`target_power_control: true`, `target_current_sense: true`): Shows toggle switch, voltage readout, and current readout. Click toggles rail.
  - `Sense-Only` (`target_power_control: false`, `target_current_sense: true`): Shows voltage and current readouts, but hides the toggle switch and removes toggle click listeners.
  - `Direct-Rail` (`target_power_control: false`, `target_current_sense: false`): Shows static rail label (e.g., `3.3V Fixed Rail`), hides current/power values, and disables the toggle switch.

### 3. Firmware Telemetry Representation
- **Choice**: In `firmware-rs/src/telemetry.rs`, gate INA180 ADC reading. For board profiles with `target_current_sense: false`, emit `null` in JSON (or 0 with `"current_sense_supported": false`).
- **Rationale**: On Pico 2, ADC 1 is exposed as external pin GP27. Attempting to calculate target current from GP27 not only displays nonsensical telemetry, but risks accidental overcurrent trips placing all pins in High-Z when GP27 sees high voltage.

### 4. App Store Requirements Matching
- **Choice**: Compare each string in `app.requires` against the boolean flags in `BOARD_MANIFEST.features` (or `BOARD_MANIFEST.zones`).
- **Rationale**: A user browsing the catalog on Pico 2 should immediately see that the `CAN Bus Analyzer` app requires an external breakout or hardware transceiver, rather than seeing a misleading `[OK]` badge.

## Risks / Trade-offs

- **[Risk] Existing DOM scripts expecting `#target-power-switch` to always be an interactive checkbox**:
  - *Mitigation*: Keep the DOM element present in the document tree, but set `display: none` and `disabled = true` when `target_power_control` is false, preventing null pointer exceptions in legacy scripts.
- **[Risk] Telemetry SSE consumers expecting integer `i_target_ma`**:
  - *Mitigation*: Maintain backward compatibility by returning `i_target_ma: 0` alongside `"target_current_sense": false` in the telemetry snapshot.
