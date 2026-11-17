## Why

In landscape viewports and wide-screen desktop displays, the default BareFOC Studio dashboard vertically stacks primary telemetry cards, motion mode selectors, the interactive radial dial, and phase vector modulation cards across three vertical tiers. This requires constant vertical scrolling to adjust setpoints while monitoring electrical angles and phase voltages. Refactoring the dashboard into a responsive two-column wide-screen layout consolidates live telemetry and vector diagnostics on the left, and motion modes and the radial setpoint dial on the right, providing an unclipped, scroll-free single-viewport experience on landscape screens.

## What Changes

- **Responsive Two-Column Dashboard**:
  - Restructure `#view-dashboard` into a responsive 12-column grid (`grid-cols-1 lg:grid-cols-12`) on wide screens.
  - **Left Column (`lg:col-span-5`)**: Telemetry and Diagnostic Hub:
    - Primary telemetry metric cards (Shaft Angle, Velocity, Target, Iq/Id current, Bus Voltage/Power, FOC Loop Rate/Temp) in a compact 2x3 or 3x2 grid.
    - Phase vector and modulation diagnostics cards (Phase Voltages U/V/W, Electrical Angles, Modulation PWM mode and limits) positioned directly below telemetry.
  - **Right Column (`lg:col-span-7`)**: Interactive Control and Motion Modes:
    - Interactive Radial Setpoint Dial with fine slider, quick step buttons, and direct numeric input.
    - Motion Control Modes (Angle Closed-Loop, Velocity Closed-Loop, Torque Mode, Velocity Open-Loop, Angle Open-Loop) and Torque Sub-Mode buttons.
- **Single-Viewport Landscape Fit**:
  - Optimize vertical padding, typography, and card spacing so the entire dashboard fits in standard 1080p and 1440p landscape displays within the `calc(100vh - 48px)` viewport without vertical scrolling.
- **Preserved Responsive Mobile Stacking**:
  - Maintain clean single-column vertical stacking on small/mobile screens (`< lg`).

## Capabilities

### New Capabilities
<!-- No new capabilities required; extending existing capability -->

### Modified Capabilities
- `native-barefoc-system-app`: Update layout and UI specifications to require responsive two-column wide-screen dashboard presentation for landscape displays.

## Impact

- `webui/barefoc.html`: Layout HTML and responsive Tailwind grid classes updated in `#view-dashboard`.
- `SYSTEM_FS`: WebUI image repackaged with `tools/pack_webui.py` and deployed to the live AgentLA hardware probe.
