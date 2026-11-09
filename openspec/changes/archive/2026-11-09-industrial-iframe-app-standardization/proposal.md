## Why

Several core system applications (`webui/terminal.html`, `webui/files.html`) and user applications (`apps/canbus`, `apps/neopixel`, `apps/servo`) exhibit layout, theme, and control usability defects when loaded inside the persistent iframe application shell (`persistent-iframe-app-shell`) of the AgentLA launcher. Key issues include:
1. Floating glass dock overlap: Critical bottom-anchored action bars (such as the terminal command prompt, CAN frame injector, and file manager action controls) are occluded by the system dock.
2. Embedded mode top bar suppression: Top navigation headers are stripped in iframe mode (`html.is-embedded #top-bar`), causing apps like Neopixel and Files to lose critical action controls, partition switches, or display duplicate navigation bars.
3. Inconsistent styling: User applications still rely on legacy dark palettes (`#0b0f19`) and inconsistent button geometries rather than conforming to the universal industrial design system (`nothing` dark theme, `industrial-light`, NDot57 typography, 16px technical dot-grid canvas, and zero emojis).

Standardizing these 5 applications ensures high-density technical usability in both standalone and embedded launcher modes.

## What Changes

- **Serial Terminal (`webui/terminal.html`)**:
  - Invert terminal layout: relocate `#cmd-input` and `#btn-send` into a sticky top-anchored sub-toolbar directly beneath the connection/baud bar.
  - Reverse chronological stream: render incoming log lines top-down (newest lines on top immediately beneath the input bar), eliminating bottom auto-scroll lock.
  - Add in-app sub-toolbar containing live connection status, baud rate, pin assignments, reset target, and clear buttons.
  - Apply 72px bottom clearance padding (`pb-[72px]`) to prevent dock occlusion.
- **File Manager (`webui/files.html`)**:
  - Assign `id="top-bar"` to the top navbar so it is cleanly suppressed in embedded mode.
  - Move partition selector (`user:/` vs `sys:/`) from the header into the main file manager sub-toolbar.
  - Add 72px bottom margin to `#file-canvas`.
- **CAN Bus Sniffer (`apps/canbus/index.html`)**:
  - Relocate TX Frame Injector (`#tx-frame-card`) into a fixed top Bento card directly below the bitrate/filter toolbar.
  - Apply universal industrial styling (`data-style="industrial"`, `data-theme="nothing"`, NDot57 header typography, dot-grid canvas).
  - Add 72px bottom clearance to packet inspector table.
- **Neopixel Controller (`apps/neopixel/index.html`)**:
  - Move *Start Shaders*, *Blackout*, FPS counter, and Power telemetry pill from `#top-bar` into a dedicated in-app sub-toolbar.
  - Apply universal industrial styling (`data-style="industrial"`, `data-theme="nothing"` / `industrial-light`, NDot57 font).
  - Add 72px bottom clearance.
- **Servo Master (`apps/servo/index.html`)**:
  - Move Gamepad status, Calibration Zero, and Emergency Disarm into an in-app sub-toolbar.
  - Refactor multi-channel sliders into high-density industrial Bento cards.
  - Add 72px bottom clearance.
- **Universal Standards Enforcement**:
  - Zero emojis across all UI labels, tooltips, console logs, and comments.
  - Synchronize theme events (`agentla:theme-change`) between host shell and embedded child frames.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `in-browser-serial-terminal`: Invert layout with top-anchored command input, reverse-chronological log stream, and dedicated sub-toolbar.
- `bento-file-manager`: Clean top-bar suppression in embedded mode and relocate partition selector into explorer sub-toolbar.
- `can-bus-analyzer-application`: Elevate TX Frame Injector to top Bento card and adopt industrial design system.
- `neopixel-control-application`: Relocate shader and blackout controls into dedicated in-app sub-toolbar and enforce industrial styling.
- `servo-master-application`: Relocate calibration and gamepad controls into dedicated in-app sub-toolbar with industrial styling.

## Impact

- Affected files:
  - `webui/terminal.html`
  - `webui/files.html`
  - `apps/canbus/index.html`
  - `apps/neopixel/index.html`
  - `apps/servo/index.html`
- Asset packaging:
  - Re-pack user applications into `apps/packages/*.tar` via `python3 tools/build_catalog.py`.
  - Re-pack system WebUI into `build/webui.tar` via `python3 tools/pack_webui.py`.
- Regression testing:
  - `test/test_can_sniffer_app.py`
  - `test/test_servo_master_app.py`
  - `test/test_neopixel_app.py`
  - `test/test_webui_assets.py`
