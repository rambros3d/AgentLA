## Why

The primary observation view in AgentLA is currently named "Active Bench" and divides digital logic analysis and analog oscilloscope functions into separate, mutually exclusive sub-modes toggled by a binary switch (`[ Logic ] [ Scope ]`). When observing mixed-signal systems—such as inspecting an I2C or SPI digital bus while simultaneously watching an analog 3.3V power droop, ADC sensor ramp, or DAC output—engineers cannot observe both domains together on a unified timeline.

Furthermore:
1. Available probe pins are not clearly surfaced; users must dig into channel selectors or the full-screen pin matrix to reassign capture channels.
2. The observation toolbar packs over 20 controls and 6 dividers into a single crowded horizontal row.
3. The protocol decoder settings are buried in a bottom toolbar button (`Decode...`), opening an awkward popup that obscures the waveforms.
4. The bottom Stimulus Deck (which only contains the Signal Generator) takes up 35% of the vertical screen, crowding the waveform display.
5. In pin configuration, voltage thresholds are mistakenly associated with digital GPIO pins, despite RP2350 digital inputs having fixed hardware CMOS Schmitt-trigger switching levels.

Renaming "Active Bench" to **"Scope"**, establishing a **Unified Mixed-Signal Oscilloscope (MSO) window** with an interactive top pin bar, 18-color preset palette, a draggable digital/analog divider, an integrated decoder bar above the digital scope, and a collapsible Signal Generator footer drawer delivers a commercial-grade, streamlined instrument experience.

## What Changes

- **Workbench Mode Rename ("Active Bench" $\rightarrow$ "Scope")**:
  - Rename the primary instrument mode from "Active Bench" to **"Scope"** across desktop navigation (left vertical sidebar -90° rotated word **SCOPE**), mobile bottom dock, view header titles, and hotkey tooltips (`Scope (Hotkey: 1)`).
- **Interactive Top Pins Bar (GP10–GP29)**:
  - Render all configurable probe pins (GP10–GP29) as quick-access buttons across the top of the Scope window.
  - Color-coded status dots: Digital pins display their active channel trace color; Analog pins display a distinct diamond icon `◈` and `ADC` tag; unconfigured pins display a muted gray outline.
  - Clicking any pin button opens the **Interactive Pin Configuration Modal** (`#modal-pin-config`).
- **Interactive Pin Configuration Modal (`#modal-pin-config`)**:
  - Operating mode selection: `Digital Input (Logic Analyzer)`, `Analog Input (Oscilloscope ADC)` (validated and enabled strictly for ADC-capable pins GP26–GP29 / ADC0–ADC3), or `Released / High-Z`.
  - Custom channel label field (e.g. `I2C_SDA`, `V_SENSE_3V3`).
  - **Curated 18 Preset Trace Colors Palette**: Clean 6 × 3 swatch grid of distinct, high-contrast colors (no complex color wheels).
  - **Hardware-Accurate Trigger Rules**: Digital pins expose only logical edge/state conditions (`Rising Edge ⤊`, `Falling Edge ⤋`, `Either Edge ⇅`, `High 1`, `Low 0`, `None`), eliminating misleading voltage thresholds. Analog ADC pins expose calibrated voltage threshold comparators.
- **Unified MSO Observation Window**:
  - Replace binary Logic/Scope toggling with a single unified MSO window:
    - **Top Subwindow (Digital Scope)**: Multi-lane stacked layout (Sigrok / PulseView style) where each digital input occupies its own private horizontal lane with in-lane frequency and duty cycle measurements (`[12.5kHz · 50%]`).
    - **Draggable Horizontal Splitter Divider (`.mso-window-splitter`)**: Tactile horizontal bar with grip handle `⋯` allowing users to drag up/down to adjust relative heights between digital and analog scopes, with persistent memory in `localStorage`.
    - **Bottom Subwindow (Analog Scope)**: All active analog channels (`CH0`–`CH3` / `GP26`–`GP29`) are superimposed on a **shared 8 × 10 division graticule** with calibrated `Volts/div` (0–3.3V), `Time/div`, and an interactive right-margin trigger level cursor (`◀ Trig`).
    - **Dynamic Visibility**: The Analog Scope subwindow is rendered **only** when at least one analog pin is active. When zero analog pins are configured, the Digital Scope automatically occupies 100% of the window height.
- **Integrated Protocol Decoder Bar (Above Digital Scope)**:
  - Position the decoder bar directly above the digital waveform lanes (`[ ☍ Decoder: I2C (SDA: D0, SCL: D1)  ▾ ]`).
  - Time-aligned in-canvas packet bubbles (`START`, `ADDR: 0x50`, `ACK`, `DATA: 0x33`, `STOP`) with hover tooltips and click-to-zoom interaction.
  - Quick transaction search filter and slide-out **Decoded Packet Table** (`[ Packet Table ]`).
- **Clustered Toolbar & Collapsible Signal Generator Drawer**:
  - Group toolbar controls into 3 visual clusters: Capture (`Run/Stop`, `Sample Rate`, `Trigger`), Analysis (`Export`, `Auto-Sniff`), and Hardware Bank claim info (`Bank: GP10–GP17`).
  - Floating zoom controls (`+`, `−`, `Fit`) inside the top-right corner of the canvas.
  - Transform the bottom Stimulus Deck into a **Collapsible Bottom Drawer**: minimized by default as a sleek footer bar (`[ Signal Generator: GP12 @ 12.0 kHz (50% PWM) ▴ ]`), expanding upward on demand to free up 90%+ vertical canvas height.

## Capabilities

### Modified Capabilities
- `unified-active-bench`: Renamed to Scope; replaces binary view toggling with a unified MSO window (digital top, analog bottom with draggable splitter), clustered toolbar, floating canvas zoom, and collapsible Signal Generator drawer.
- `adaptive-workspace-hub`: Updates primary mode navigation label from "Active Bench" to "Scope" across desktop left vertical sidebar (-90° rotated word "SCOPE") and mobile bottom dock.
- `webui-protocol-overlays`: Moves protocol decoder settings to an integrated bar directly above the digital scope lanes, with packet hover tooltips and interactive packet table.

### New Capabilities
- `probe-pin-configuration`: Top available probe pins bar (GP10–GP29) with status dots and interactive pin configuration modal featuring the 18 preset trace color palette, custom labels, and hardware-accurate digital edge / analog voltage trigger rules.

## Impact

- **Affected Files**:
  - `firmware/html/index.html` (rename Active Bench to Scope, top pin bar markup, unified MSO canvas layout with draggable splitter, clustered toolbar, decoder bar above digital scope, `#modal-pin-config` markup, collapsible signal generator drawer)
  - `firmware/html/style.css` (top pin bar styling, 18-color swatch grid, unified MSO layout, draggable splitter bar styles, decoder bar above scope, floating zoom controls, collapsible drawer animations)
  - `firmware/html/app.js` (mode switching update for "scope", top pin bar click handlers, `#modal-pin-config` controller with 18 preset colors, unified MSO canvas sizing and dual rendering loop, draggable splitter mouse/touch handlers, decoder bar and packet table handlers)
- **Zero Flash/Firmware C Regressions**: Strictly frontend HTML/CSS/JS assets; zero changes to C microcontroller drivers.
