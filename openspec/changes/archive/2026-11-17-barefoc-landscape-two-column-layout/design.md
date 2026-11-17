## Context

In the native BareFOC Studio workbench (`webui/barefoc.html`), the Dashboard view (`#view-dashboard`) is structured into three vertically stacked blocks:
1. Top horizontal row of 6 telemetry HUD cards.
2. 12-column sub-grid containing the Motion Modes list (col-span-5) and the Setpoint Dial (col-span-7).
3. Bottom horizontal row of 3 cards for Phase Voltages, Electrical Angles, and Modulation Mode.

On standard 16:9 and 16:10 desktop and laptop displays (1366x768 to 1920x1080), this 3-tier vertical stack exceeds the viewport height (`calc(100vh - 48px)`), forcing continuous vertical scrolling whenever an operator adjusts motion setpoints while monitoring phase voltages or electrical angles.

## Goals / Non-Goals

**Goals:**
- Restructure `#view-dashboard` into a responsive 2-column layout on wide screens (`lg:` breakpoint and above):
  - **Left Column (`lg:col-span-5`)**: Telemetry and Phase Vector Diagnostics.
    - Top: 6 Primary Telemetry cards (Angle, Velocity, Target, Iq/Id, Bus Voltage/Power, Loop Rate/Temp) in a compact 2x3 grid.
    - Bottom: Phase Voltages, Electrical Angles, and Modulation Mode cards stacked cleanly below telemetry.
  - **Right Column (`lg:col-span-7`)**: Interactive Control & Motion Modes.
    - Top: Motion Control Modes & Torque Sub-modes card.
    - Center/Bottom: Interactive Radial Setpoint Dial, Range Slider, Quick Step buttons, and Direct Numeric Input.
- Ensure the complete dashboard fits within the visible landscape viewport without vertical scrollbars.
- Preserve responsive single-column stacking on mobile or narrow displays (`< lg`).
- Maintain zero emojis and strict styling compliance (no CSS dot indicators, standard DaisyUI components).

**Non-Goals:**
- Changing underlying protocol adapters (Commander, Fibre, MultiBus, CAN).
- Altering other view panels (Tuning, Scope, Wizard, Console), which already fit within their respective viewports.

## Decisions

### D1: Asymmetric 5/7 Split Grid
- **Decision**: Use Tailwind's `lg:grid-cols-12` with `lg:col-span-5` for the left telemetry/diagnostics column and `lg:col-span-7` for the right actuation column.
- **Rationale**: The interactive radial dial and motion mode cards require greater horizontal width for circular SVG gauge rendering and button layouts, whereas telemetry metrics and phase voltage readouts format cleanly into dense 2-column cards.

### D2: Preserving Element IDs for Zero JS Regression
- **Decision**: Keep all existing element IDs (`#hud-angle-val`, `#hud-vel-val`, `#foc-dial`, `#range-target-slider`, `#btn-send-numeric-target`, `#val-ua`, `#val-theta-e`, etc.) identical.
- **Rationale**: All telemetry polling and UI updates in `barefoc.html` rely on these specific IDs. Keeping them unchanged guarantees 100% JavaScript compatibility with zero logic regressions.

## Risks / Trade-offs

- **[Risk] Compact telemetry cards feeling cramped on intermediate screen widths (1024px-1280px)**
  - **Mitigation**: Use compact padding (`p-2.5` to `p-3`), dense text hierarchy (`text-[10px]` uppercase labels, `text-base` / `text-lg` metric numbers), and flexible grid columns (`grid-cols-2 sm:grid-cols-3 lg:grid-cols-2 xl:grid-cols-3`).
