## Context

AgentLA's WebUI is an integrated instrumentation workspace designed for physical electronics debugging. Following the desktop navigation streamlining (relocating workbench mode tabs to a left vertical sidebar with -90° rotated word labels) and console text wrapping enhancements, four primary areas remain cluttered or visually disconnected:
1. **Luminance Inversion & Missing Neumorphism in Dark Theme**: Dark Mode ("Forest") uses `--color-base-100: oklch(26% 0 0)` for the page canvas and `--color-base-200: oklch(20% 0 0)` for card surfaces. Because cards have lower luminance than the canvas, they appear sunken/recessed rather than elevated. Furthermore, ambient drop shadows are missing and the specular rim catch highlight is only 8% opacity, reducing dark mode to a flat wireframe.
2. **Top-Bar Target Power Pill**: The Target Power Rail (`VTarget`) and live INA180 current sensing are confined inside the Debugger view, despite power being a global requirement across all modes.
3. **Overcrowded Debugger View**: The presence of the power card squeezes the Universal Flasher and SWD Debugger into a cramped vertical space.
4. **Adapter Grid Overload**: All 6 hardware bridges (UART, I2C, I3C, SPI, CAN, Industrial) are rendered simultaneously in a massive grid with 6 open logs, creating cognitive friction.
5. **Observation Toolbar & Signal Generator Congestion**: In the Active Bench, 20 controls are crammed into a single toolbar, while the bottom Stimulus Deck consumes 35% of vertical space for a simple PWM clock generator.

## Goals / Non-Goals

**Goals:**
- Unify tactile neumorphic depth across Dark and Light themes by aligning luminance hierarchies and applying a dual-shadow formula (ambient drop shadow + specular rim highlight).
- Relocate VTarget power control and INA180 current sensing into a live top-bar toggle pill (`[ ⚡ 3.31V · 14mA ]`).
- Reorganize the Debugger tab into an uncluttered two-column layout: Universal Flasher on the left (~400px), SWD / CMSIS-DAP Debugger & Memory Viewer on the right (`flex: 1`).
- Transform the USB Protocol Adapters view into an intuitive Master-Detail layout with persistent per-protocol logs, global auto-sniffing, profile-aware default selection, and quick transaction preset chips.
- Streamline the Active Bench observation bar into 3 functional clusters (Capture, Analysis, Bank Info), float canvas zoom controls, and turn the Signal Generator into a collapsible bottom drawer footer.

**Non-Goals:**
- Modifying C microcontroller backend firmware or USB descriptors (the existing `/api/power` and `/mcp` endpoints already support all needed actions).
- Removing any protocol adapters or signal generator capabilities.
- Introducing external CSS/JS framework dependencies.

## Decisions

### 1. Unified Neumorphic Depth System
- **Luminance Hierarchy**:
  - In Light Mode ("Emerald"): Page canvas is `oklch(93% 0 0)`, cards are `oklch(100% 0 0)` (elevated).
  - In Dark Mode ("Forest"): Page canvas is updated to `oklch(18% 0 0)`, cards to `oklch(24% 0 0)` (elevated).
- **Dual-Shadow Formula**:
  - Light Theme: `--card-neumorphic-shadow: 0 4px 14px rgba(0, 0, 0, 0.08), inset 0 1px 0 rgba(255, 255, 255, 0.85);`
  - Dark Theme: `--card-neumorphic-shadow: 0 6px 18px rgba(0, 0, 0, 0.65), inset 0 1px 0 rgba(255, 255, 255, 0.16);`
- **Soft Translucent Borders**: `--border-color` is set to `rgba(255, 255, 255, 0.08)` in dark mode and `rgba(0, 0, 0, 0.12)` in light mode, preventing harsh wireframes.

### 2. Top-Bar Target Power Pill
- **Placement**: Directly to the left of the `[SAFE STATE]` button in `#top-bar`.
- **States**:
  - **Rail ON**: Soft emerald green tint (`color-mix(in oklch, var(--color-success) 14%, var(--color-base-200))`), pulsing status dot, live telemetry `3.31V · 14mA`. Click action dispatches `POWER OFF`.
  - **Rail OFF**: Neutral background, muted dot, `POWER OFF` label. Click action dispatches `POWER ON`.
  - **Overcurrent Warning**: Pulsing amber/red alert pill if current exceeds threshold.
- **Telemetry Loop**: Polled alongside the 1-second watchdog heartbeat or updated via SSE events.

### 3. Debugger View Two-Column Layout
- With the power card moved to the top bar, `#view-target-station` adopts a clean two-column grid:
  - **Left Column (~400px)**: Universal Target Flasher card with drag-and-drop firmware dropzone, chip selector, base address, flash progress bar, and flasher terminal log.
  - **Right Column (`flex: 1`)**: SWD / CMSIS-DAP Debugger with target detection badge, execution control bar (`Halt`, `Resume`, `Step`, `Reset`), core registers grid (`R0-R15`, `SP`, `LR`, `PC`), and live memory hex inspection table.

### 4. Master-Detail USB Protocol Adapters Layout
- **Left Master List (~280px)**: Real-time status cards for all 6 protocols (UART, I2C, I3C, SPI, CAN, Industrial). Shows active status dot (`●` green when active, `○` gray when idle), protocol name, active baud/clock rate, and claimed hardware pins.
- **Right Detail Pane (`flex: 1`)**:
  - Active protocol configuration fields (baud rate, addresses, frequencies).
  - Transaction controls (Write, Read, Write+Read, Scan).
  - Quick preset chips (`Scan Bus`, `Read 24C02 EEPROM`, `Ping OBD-II`).
  - High-visibility communication terminal log.
- **Persistent DOM Containers**: Each protocol maintains its own terminal log container (`#uart-log`, `#i2c-log`, `#can-log`, etc.), ensuring zero dropped packets or cleared scrollback during switching.
- **Smart Protocol Selection**: Defaults to UART Bridge on fresh load, or automatically opens the primary protocol claimed by the active hardware profile.
- **Global Auto-Sniff Toggle**: Header-level checkbox automatically arms the Logic Analyzer to capture and decode transmitted bus packets.

### 5. Streamlined Active Bench & Collapsible Signal Generator Drawer
- **Clustered Toolbar**:
  - **Capture Cluster**: `[ ▶ RUN / STOP ]` + `Rate: [ 10 MS/s ▾ ]` + `Trigger: [ D0 ⤊ ▾ ]`.
  - **Analysis Cluster**: `[ ☍ Decode ▾ ]` + `[ ⇪ Export ▾ ]` + `Auto-Sniff`.
  - **Hardware Claim Info**: `Bank: GP20–GP27 (8 Ch)`.
- **Floating Canvas Zoom Controls**: Zoom in (`+`), Zoom out (`−`), and Zoom fit (`⊡`) are placed inside a compact, semi-transparent floating control block in the top-right corner of `#waveform-canvas-wrap`.
- **Collapsible Signal Generator Drawer**:
  - Minimized by default as a sleek bottom footer bar (`[ ⚡ Signal Generator: GP12 @ 12 kHz (50% PWM) ▾ ]`).
  - Expands upward on click to expose pin selection, frequency input, duty cycle slider, and output toggle, leaving 90%+ of vertical height for waveform analysis.

## Risks / Trade-offs

- **[Risk] User unaware of Signal Generator footer drawer**: A user looking for the signal generator might not notice the compact bottom bar.
  - **Mitigation**: The footer bar displays a clear bolt icon, live frequency/pin summary, and an expand chevron `▴` with tooltip `Click to configure Signal Generator`.
- **[Risk] Accidental power toggle**: Single-clicking the top-bar power pill could cut power to an active target unintentionally.
  - **Mitigation**: The button requires a deliberate click (hotkeys require Shift modifier), displays a clear hover tooltip with current state, and the emergency `SAFE STATE` button remains distinct with red accenting.
