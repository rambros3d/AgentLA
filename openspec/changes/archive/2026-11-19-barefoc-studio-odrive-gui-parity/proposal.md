## Why

BareFOC Studio (`webui/barefoc.html`) is a powerful but hard-wired motor workbench: its controls, tuning fields, wizard inputs, and plot traces are fixed in markup, and its parameters are not discoverable from the target. The ODrive GUI reference in `ref/ODrive/GUI` is far more intuitive — it auto-discovers devices and axes, exposes the full typed parameter tree, lets users build dashboards from any parameter, provides a true stepped setup wizard with calibration gating and human-readable error decoding, and supports multi-device/multi-axis operation. This change brings BareFOC Studio to feature and UX parity with ODrive GUI while preserving and extending its barefoc-specific instrumentation (FOC internals, DRV83xx gate-driver tooling, step-response analyzer, Emergency Safe, CAN/UART/Fibre/MultiBus bridges). The result is a single near-lookalike experience that is approachable for onboarding and still the most capable tool for barefoc debugging.

## What Changes

- **Device & axis discovery**: Add auto-discovery of connected motor targets and their axes, with a persistent multi-device/multi-axis status footer (one chip per axis) modeled on ODrive GUI's `Axis.vue` — color-coded connected/error states, click-to-expand error popups decoding axis/motor/encoder/controller fault bits into human-readable names, and per-axis Clear Errors.
- **Dynamic parameter explorer**: Discover the live typed parameter/function tree from the target (Fibre endpoint-0 schema, MultiBus register map, Commander capabilities) and render generic controls (boolean, numeric, enum, slider, function) from it, so any exposed parameter is controllable without editing HTML. Preserve read-only flags and numeric typing.
- **Customizable dashboards**: Let users add/remove controls, sliders, functions, actions, and plots from a parameter tree; support multiple named dashboards; export/import dashboard layout JSON and full target configuration JSON.
- **Guided setup wizard**: Rebuild the current flat Hardware Setup view as a stepped, state-aware wizard (Back/Next/Finish, progress navigation, per-choice tooltips, completion gating) covering multi-axis motor and encoder selection/calibration, control-mode selection, misc limits, and a Review & Apply diff page. Include real motor calibration and encoder calibration routines with status feedback and expected-error masking during calibration.
- **Arbitrary-variable oscilloscope**: Extend the scope so any discovered parameter can be selected per trace; keep the existing 2-trace canvas, timebase, autoscale, freeze, crosshair, CSV export, and step-response analyzer.
- **Fix existing stubs and dead controls**: Wire "Read From Target" tuning sync, modulation/supply/voltage/velocity limit inputs, scope crosshair, target power toggle, and real endpoint-0 schema fetch; replace timer-based alignment with hardware-feedback-driven parsing.
- **BareFOC-only extensions retained and promoted**: Gate-driver register read/write, electrical-zero alignment, phase-voltage diagnostics, emergency-safe state, virtual simulator, and all existing transports remain available through the new generic parameter system.
- **BREAKING** (UI behavior only): The five fixed tabs (`Live Studio`, `Controller Tuning`, `Oscilloscope`, `Hardware Setup`, `Diagnostics & Console`) are replaced by ODrive-GUI-style named dashboards, a dedicated stepped Wizard page, a persistent axis footer, and a consolidated configurable dashboard. All underlying protocol behavior, routes (`/motor`, `/foc`, `/barefoc`), and system packaging remain unchanged.

## Capabilities

### New Capabilities
- `motor-target-discovery`: Auto-discovery of connected motor targets and their axes, connection state tracking, and per-axis status/error decoding with Clear Errors.
- `motor-parameter-explorer`: Discovery of the live typed parameter/function tree from a target and dynamic rendering of generic typed controls from it.
- `motor-dashboard-builder`: User-configurable dashboards (add/remove widgets and plots from the parameter tree, named dashboards, JSON export/import of layouts and full target configuration).
- `motor-setup-wizard`: Stepped, state-aware multi-axis setup wizard with motor/encoder selection, hardware-driven calibration routines, completion gating, and a Review & Apply diff.
- `motor-arbitrary-waveform-plotting`: Selection of arbitrary discovered parameters as oscilloscope traces while retaining CSV export and step-response analysis.

### Modified Capabilities
- `native-barefoc-system-app`: Adds requirements for ODrive-GUI-style shell (named dashboards, persistent axis footer, configurable workspace), dynamic parameter-driven controls, multi-device/multi-axis support, stepped wizard, arbitrary-variable scope, and wiring of previously stubbed controls. Existing packaging/route/nav/transport requirements are retained.
- `barefoc-studio-application`: The legacy USER_FS-oriented spec is stale (app is system-only). Its behavior requirements are superseded by `native-barefoc-system-app` and the new capabilities above; this change records that supersession rather than adding new USER_FS packaging requirements.

## Impact

- **Code**: Primary implementation in `webui/barefoc.html` (shell rework, new discovery/parameter/dashboard/wizard/scope engines, stubs fixed). No new Rust crate; FOC loops remain on external targets.
- **Firmware routes / packaging**: Unchanged. `/motor` and aliases in `firmware-rs/src/web.rs`, `tools/pack_webui.py`, and `tools/build_catalog.py` continue to serve `barefoc.html` from SYSTEM_FS.
- **Bridge APIs**: Continues to use `AgentLA.i2c.*`, `AgentLA.spi.transfer`, `AgentLA.telemetry.emergencySafeState`, `window.FeatureRegistry`. Introduces discovery using existing WS bridges (`/api/v1/can/ws`, `/api/v1/terminal/ws`) and REST bridges (`/api/v1/bus/i2c/transfer`, `/api/v1/bridge/spi/transfer`). A power-control API (`AgentLA.power.setEnabled`) may be added to `webui/agentla-bridge.js` if firmware support exists; otherwise the existing target power toggle remains guarded by `FeatureRegistry`.
- **Tests**: `test/test_webui_assets.py` assertions for two-column layout and zero emojis must remain valid or be updated for the new shell. New tests should cover discovery, dynamic controls, wizard flow, and footer error decoding.
- **UI standards**: Must comply with AGENTS.md — zero emojis, no CSS dot indicators, dock anchored to bottom, full-bleed viewports `calc(100vh - 48px)`, launcher centering, daisyUI/Tailwind styling.
