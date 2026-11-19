## Context

BareFOC Studio is a single self-contained `webui/barefoc.html` (approx. 3,000 lines) served as a SYSTEM_FS app at `/motor`. It currently implements five fixed tab views (Live Studio, Controller Tuning, Oscilloscope, Hardware Setup, Diagnostics & Console) with hand-coded controls, a 2-trace canvas scope, a 4-step alignment wizard, and multiple protocol adapters (Commander ASCII, Fibre binary RPC, CAN WS, MultiBus I2C/SPI). The ODrive GUI reference (`ref/ODrive/GUI`) uses a Vue 2 + Vuex + Python Flask/socket.io architecture to offer auto-discovery, a full typed parameter tree, configurable dashboards, a stepped multi-axis wizard, arbitrary-variable plots, and a persistent per-axis error-decoding footer. The user wants BareFOC Studio to be a near-lookalike of that ODrive GUI UX, with full multi-device/multi-axis support, all parity phases implemented sequentially, and a formal spec-first workflow.

Constraints (from AGENTS.md and repo reality):
- Everything remains in one HTML file; no new JS framework or build step is introduced for this app. DaisyUI/Tailwind classes are the styling system; no CSS dot indicators; zero emojis; dock anchored bottom; full-bleed viewports; launcher centered.
- Firmware is `#![no_std]`; no FOC loop runs on AgentLA. The app is a client-side workbench only.
- Existing routes, packaging (`pack_webui.py`, `build_catalog.py`), OTA rules, and LittleFS boundaries are unaffected.
- Several existing controls are stubs (`btn-sync-tuning`, modulation/limit inputs, scope crosshair, power toggle, endpoint-0 schema fetch) and must be wired, not left decorative.

## Goals / Non-Goals

**Goals:**
- Achieve feature and UX parity with ODrive GUI: discovery, full parameter tree, configurable dashboards, stepped wizard with calibration, multi-device/multi-axis status footer with error decoding, arbitrary-variable plotting, and config/layout import-export.
- Preserve and expose all barefoc-specific tooling (FOC internals, DRV83xx registers, step-response analyzer, Emergency Safe, all transports, simulator) through the new generic system.
- Support multiple connected targets, each with multiple axes, mirroring ODrive GUI's `odrive0.axis0` model.
- Fix all known stubs and dead controls so every visible control performs a real action.

**Non-Goals:**
- Introducing Vue/React/Electron or a Python backend for this app. Remains vanilla JS in `barefoc.html`.
- Running FOC control loops on AgentLA hardware or changing any Rust firmware crate.
- Re-adding USER_FS app packaging (the legacy `barefoc-studio-application` spec is stale; app is SYSTEM_FS only).
- Adding firmware-update capability (ODrive GUI itself has none).

## Decisions

**D1 — Single-file vanilla JS, no framework (keep).** The app stays a single HTML file with inline styles and scripts, using DaisyUI/Tailwind utility classes and the existing system assets (`/daisyui.css`, `/style.css`, `/themes.css`, `/agentla-bridge.js`). Rationale: the file is already large and self-contained; adding a framework would require a build pipeline and conflict with ROM packaging. Alternative (Vue 2 clone) rejected because it would duplicate the reference architecture without deployment benefit.

**D2 — New shell mirroring ODrive GUI's App.vue structure.** Replace the five fixed tab panels with: (a) a top header of named dashboards with a Menu dropdown for Export/Import dash and Import/Export target config, plus a global "start/stop sampling" toggle; (b) a dynamic view area rendering either a Dashboard, the Wizard, or a Start/connection page; (c) a persistent bottom axis-status footer. The existing connection/safety toolbar (protocol/transport/baud/node, connect, Emergency Safe, power toggle) remains above the view. Rationale: this is the interaction model the user prefers; alternatives (keep tabs and bolt features on) rejected because they would not deliver the requested "near-lookalike" UX.

**D3 — Centralized state store (Vuex-like) in plain JS.** Introduce a `StudioStore` object with `state`, `commit`/`dispatch`-style methods, and an event bus replacing scattered globals. It owns: discovered targets/axes, connection states, per-target parameter trees (typed values + readonly flags + functions), derived views (params, writable, writable numeric, functions), dashboards (controls/actions/plots), sampled series buffers, wizard config, and error codes. Rationale: ODrive GUI's parity features (dynamic controls, configurable dashboards, footer decoding) all depend on one reactive source of truth; current ad-hoc `state` object is insufficient. Alternative (keep globals and duplicate) rejected as unmaintainable.

**D4 — Parameter discovery is transport-aware and progressive.** Build a `ParameterExplorer` that obtains a typed parameter tree from each active adapter:
- Fibre: real endpoint-0 read (fix stub) returns JSON schema → normalize to `{val, readonly, type}` nodes and functions.
- MultiBus: known register descriptor map (TARGET/ENABLE/MODE/ANGLE/VELOCITY/CURRENT_Q/PIDs/limits) expanded to a tree; I2C/SPI reads populate values.
- Commander: capability probe via `?`/`M1 C` and known SimpleFOC command set → synthesize a conservative tree; read-only telemetry fields flagged.
Each tree is cached per target and refreshed on connect and on explicit "Refresh". Node paths use `target.axisN.<...>` naming compatible with existing adapters. Rationale: a single discovery interface hides protocol differences and makes dynamic controls possible for every transport. Alternative (hard-code one tree) rejected because it recreates the current rigidity.

**D5 — Generic control components generated from the tree.** Implement renderers for: boolean (checkbox), numeric (input with math evaluation + unit display), enum (select from known enum tables, extended from `odriveEnums.json`-style mapping), slider (min = value/4, max = value×4, adsorbing steps), function (button invoking `M1 W`/`save_configuration`-style call), action (deferred set with Send). Each node gets Add-to-dash controls and an X remove. Read-only nodes render value-only. Rationale: mirrors ODrive GUI's Ctrl* components and is the key to "any parameter is controllable".

**D6 — Dashboard builder with JSON import/export.** Dashboards are plain objects `{name, id, controls[], actions[], plots[]}` stored in the store and serialized to JSON. Menu actions export the current dashboard or all dashboards; import parses JSON and re-registers sampled plot variables. A default "Tuning" dashboard is auto-created on first target connection (mirroring ODrive GUI's `Tuning_0_5_1.json`) containing current/velocity limits, pos/vel/integrator gain sliders, a requested-state enum, a save function, two setpoint actions, and two command-vs-estimate plots. Rationale: gives the familiar ODrive GUI workflow out of the box.

**D7 — Multi-device/multi-axis discovery and footer.** A `Discovery` service probes active transports for targets and axes (CAN heartbeat node IDs, MultiBus I2C addresses 0x40+node, Fibre endpoint-0 discovery, UART commander presence) and emits target/axis list updates. The footer renders one chip per `target.axisN`, color-coded grey (disconnected), green (ok), red (error); clicking opens a popup listing axis/motor/encoder/controller error names decoded from bitmasks via an error dictionary, plus a Clear Errors button that writes 0 to all four error fields. Error polling runs every 1000 ms like ODrive GUI. Rationale: directly delivers the user's multi-device/multi-axis requirement and ODrive GUI's most distinct UX element.

**D8 — Stepped wizard with calibration gating.** Rebuild Hardware Setup as a page graph like `wizard.js`: Target → Brake/Supply → per-axis Motor (model presets + motor calibration button + status) → Encoder (type presets with computed CPR, e.g. hall = 6×pole pairs + encoder calibration + status) → Control mode → Misc limits → (repeat for axis1) → Review & Apply diff. Nav shows links with active/finished states; Next is gated on `choiceMade`; tooltips explain choices; calibration temporarily masks expected errors (mirroring `Axis.vue` masking). Calibration actions call real adapter routines (motor/encoder calibration requested states or SimpleFOC routines), not timers. Rationale: this is the core of ODrive GUI's intuitive onboarding and addresses the user's "proceed with all sequentially" directive.

**D9 — Arbitrary-variable scope.** Replace fixed `sel-trace-a/b` option lists with a populated list of all sampled/discoverable numeric parameter paths. The user adds any parameter to sampling; scope traces bind to those paths. Ring buffer, timebase, autoscale, freeze, crosshair (fix the currently decorative overlay), CSV export, and step-response analyzer are retained. Sampling is global and started/stopped from the header. Rationale: matches ODrive GUI's plot-any-variable model while keeping barefoc's superior analyzer.

**D10 — Stub remediation.** Wire `btn-sync-tuning` to read gains from the target/persisted config; wire modulation/supply/voltage/velocity limit inputs to parameter writes; implement scope crosshair pointer handlers; implement power toggle through `FeatureRegistry` guard and an optional bridge `power` API if available; replace timer-based alignment with parsing of real target responses; drive fault badges from decoded error bits. Rationale: a parity app cannot ship visible dead controls.

**D11 — Sequential implementation with tests.** Work in phases A (shell + footer + discovery), B (parameter explorer + dashboard builder), C (wizard + calibration), D (scope + stub fixes), matching the user's "proceed with all of them sequentially" answer, with a testable acceptance scenario per capability and updates to `test/test_webui_assets.py`.

## Risks / Trade-offs

- [Single-file size grows substantially (likely >6k lines) → Mitigation: organize into clearly commented sections and keep pure helper functions; avoid external modules because of ROM packaging constraints.]
- [Targets may not all expose full typed schemas (Commander especially) → Mitigation: progressive discovery with a conservative known-command tree for Commander; clearly label synthesized vs discovered nodes and allow manual add/refresh.]
- [Multi-device discovery on shared UART/CAN bus may be ambiguous → Mitigation: require explicit node/address selection in connection toolbar; discovery populates candidates and user confirms; do not auto-write to all targets.]
- [Dynamic controls could accidentally write unsafe parameters → Mitigation: honor read-only flags, require explicit "Send" for actions, keep Emergency Safe and Quick Stop globally available; gate writes while motor enabled where appropriate.]
- [Calibration routines differ across SimpleFOC/ODrive/Firmware versions → Mitigation: adapter-specific calibration implementations behind one wizard interface; expected-error masking and status decode per protocol; never assume a single wire format.]
- [Existing tests assert fixed five-tab structure / two-column dashboard → Mitigation: update tests to assert new shell invariants (footer present, dashboards present, zero emojis, responsive two-column dashboard retained) rather than deleting coverage.]
- [Performance of 50 Hz telemetry plus dynamic DOM updates for many nodes → Mitigation: store-driven DOM diffing limited to visible dashboard + footer; reuse existing ring-buffer canvas rendering; avoid re-rendering whole tree each tick.]

## Migration Plan

1. Implement store and shell behind a feature flag-like `STUDIO_V2` constant so the legacy view can be toggled during development; default to new shell only after all phases pass tests.
2. Phase A: discovery + footer + shell; verify against simulator (multi-axis simulated targets) before hardware.
3. Phase B: parameter explorer + dashboard builder; import/export round-trip test with default Tuning dashboard.
4. Phase C: wizard + calibration; verify gating and diff apply against simulator and at least one physical transport (CAN or UART) with Debugprobe available as fallback.
5. Phase D: arbitrary-variable scope + stub fixes; re-run all WebUI asset tests and a manual smoke test on `/motor`.
6. Deploy with existing `tools/pack_webui.py` → `POST /api/v1/system/webui/update`; no SYSTEM_FS layout changes, so rollback is a repack of the previous `barefoc.html`.

## Open Questions

- Does firmware expose a target power control API suitable for `AgentLA.power.setEnabled`? If not, the power toggle stays FeatureRegistry-guarded and documented as unavailable.
- Should discovery auto-connect to all found targets, or populate a candidate list requiring user confirmation? Current design assumes confirmation (safer for shared buses).
- What is the canonical error-bit dictionary for barefoc targets (SimpleFOC/ODrive versions) to embed for human-readable decoding? ODrive's `odriveEnums.json` is the reference; barefoc-specific fault names must be confirmed.
