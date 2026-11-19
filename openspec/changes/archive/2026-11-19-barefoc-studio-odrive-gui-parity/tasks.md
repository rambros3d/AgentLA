## 1. Foundation: Store And Shell

- [x] 1.1 Introduce a centralized `StudioStore` in `webui/barefoc.html` with `state`, subscribe/commit helpers, and an event bus; migrate existing globals (`state`, `scope`) into it
- [x] 1.2 Build the ODrive-GUI-style shell: dashboard header tabs + Menu dropdown (Export/Import dash, Import/Export target config), dynamic view area, persistent connection/safety toolbar, and bottom axis footer; keep `/motor` routes and packaging unchanged
- [x] 1.3 Add a `STUDIO_V2` switch so the legacy five-tab view can be toggled during development; default to new shell after all phases pass tests
- [x] 1.4 Verify AGENTS.md UI compliance: zero emojis, no CSS dot indicators, dock anchored bottom, full-bleed `calc(100vh - 48px)` viewports, centered launcher, daisyUI/Tailwind styling

## 2. Phase A: Target Discovery And Axis Footer

- [x] 2.1 Implement a `Discovery` service that probes active transports (CAN heartbeat IDs, MultiBus I2C addresses 0x40+node, Fibre endpoint 0, UART commander presence) and lists candidate targets/axes with connection state
- [x] 2.2 Extend the simulator to expose at least two simulated targets with two axes each for multi-device/multi-axis testing
- [x] 2.3 Render one footer chip per discovered axis with grey/green/red status styling; keep visible on every view
- [x] 2.4 Build an error dictionary mapping barefoc/ODrive fault bits to human-readable names for axis, motor, encoder, and controller
- [x] 2.5 Implement click-to-expand error popup listing decoded error names (or 'none') plus a Clear Errors control that writes zero to all four error fields
- [x] 2.6 Add 1000 ms error polling for connected axes that updates footer and popup without user interaction
- [x] 2.7 Add discovery refresh that adds/removes targets and axes without disturbing live connections or dashboard widgets

## 3. Phase B: Parameter Explorer And Dashboard Builder

- [x] 3.1 Implement a `ParameterExplorer` with a transport-aware discovery interface returning normalized nodes `{val, type, readonly, path}` and functions
- [x] 3.2 Fibre: replace the stub `inspectSchema()` with a real endpoint-0 read, decode the schema to typed nodes, and cache per target
- [x] 3.3 MultiBus: expand the known register descriptor map (target, enable, mode, angle, velocity, current, PIDs, voltage/velocity limits) into a typed tree and populate live values over I2C/SPI
- [x] 3.4 Commander: probe `?`/`M1 C` and synthesize a conservative typed tree distinguishing writable commands from read-only telemetry
- [x] 3.5 Build a browsable parameter tree grouped by target/axis showing current values and types, with a refresh action
- [x] 3.6 Implement generic control renderers: boolean checkbox, numeric input with math evaluation and unit display, enum select, writable-numeric slider with derived min/max and adjustable bounds, function button, read-only value display, and deferred action with Send
- [x] 3.7 Implement add/remove widgets from the tree into dashboards (control, slider, function, action, plot)
- [x] 3.8 Implement multiple named dashboards: create, switch, delete, and select as active view
- [x] 3.9 Auto-create a default Tuning dashboard on first target connection with current/velocity limits, pos/vel/integrator gain sliders, requested-state enum, save function, two setpoint actions, and two command-vs-estimate plots; register plot vars for sampling
- [x] 3.10 Implement dashboard JSON export/import (widgets + plot var registration) and target configuration JSON export/import (values and types preserved, applied via adapter)
- [x] 3.11 Verify import/export round-trip against the default Tuning dashboard

## 4. Phase C: Stepped Setup Wizard And Calibration

- [x] 4.1 Define a wizard page graph (Target → Brake/Supply → per-axis Motor → Encoder → Control mode → Misc → Review & Apply) generalizing ODrive GUI's `wizard.js` for N discovered axes
- [x] 4.2 Render wizard navigation with active/completed page links, Back/Next/Finish controls, per-choice tooltips, and Next gating on `choiceMade`
- [x] 4.3 Implement per-axis motor presets and custom fields (type, pole pairs, KV/torque constant, phase R/L) writing into wizard config
- [x] 4.4 Implement per-axis encoder presets including Hall CPR derivation (6 × pole pairs), incremental with/without index, and CPR fields
- [x] 4.5 Implement control-mode and misc-limit pages (velocity/current limits) per axis
- [x] 4.6 Implement motor calibration action via real adapter routine with Calibrating/Success/Failure status and storage of measured phase R/L
- [x] 4.7 Implement encoder calibration action computing scan distance and invoking adapter routine with success/failure status
- [x] 4.8 Apply expected-error masking while wizard calibration runs (phase R/L out of range, CPR/pole-pair mismatch, no response) without affecting other views
- [x] 4.9 Implement Review & Apply page showing old-vs-new value diff with changed values highlighted, and applying all non-null changes via adapters
- [x] 4.10 Verify wizard gating, calibration status, and diff apply against simulator and at least one physical transport (CAN or UART), using the Debugprobe as recovery fallback

## 5. Phase D: Arbitrary-Variable Scope And Stub Fixes

- [x] 5.1 Populate scope trace selectors from all registered sampled numeric parameters instead of fixed options; add tree-driven parameter-to-sampling registration
- [x] 5.2 Wire header start/stop sampling to enable/disable sampling of all registered plot variables and confirm plots update
- [x] 5.3 Implement interactive crosshair pointer handlers on `#scope-canvas` (vertical line + time/value label) replacing the decorative overlay
- [x] 5.4 Wire `btn-sync-tuning` (Read From Target) to read gains and limits from the target/persisted config into the visible controls
- [x] 5.5 Wire modulation scheme, supply voltage, voltage limit, and velocity limit inputs to parameter writes over the active adapter
- [x] 5.6 Replace timer-based alignment with parsing of real zero-electrical-angle and sensor-direction responses; enable Apply & Store Calibration only on real completion
- [x] 5.7 Wire target power toggle through `FeatureRegistry.hasTargetPowerControl` and an optional bridge power API; keep it guarded when unsupported
- [x] 5.8 Drive diagnostic fault badges (NORMAL_STATE, OVER_VOLTAGE, UNDER_VOLTAGE, OVER_CURRENT, THERMAL_FAULT, SENSOR_LOSS) from decoded error bits instead of static styling
- [x] 5.9 Retain and verify CSV export, timebase buttons, autoscale, freeze/resume, FPS display, and step-response metrics (rise/settle/overshoot/error)

## 6. Tests, Docs, And Deployment

- [x] 6.1 Update `test/test_webui_assets.py` so assertions cover the new shell (footer, dashboards, parameter explorer, wizard) while preserving zero-emoji and responsive two-column dashboard checks
- [x] 6.2 Add tests for discovery/footer error decoding (simulator), parameter-node-to-control mapping, dashboard import/export round-trip, wizard gating + calibration + apply, and arbitrary trace selection
- [x] 6.3 Run the full WebUI test suite and a manual `/motor` smoke test on device, confirming no route/packaging regressions
- [x] 6.4 Repackage with `tools/pack_webui.py` and deploy via the existing `/api/v1/system/webui/update` flow (no SYSTEM_FS layout change; rollback = repack previous barefoc.html)
- [x] 6.5 Record the `barefoc-studio-application` spec supersession note in the archive when this change is archived
