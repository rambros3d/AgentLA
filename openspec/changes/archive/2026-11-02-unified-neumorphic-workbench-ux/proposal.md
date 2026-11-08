## Why

While recent updates streamlined navigation and resolved console text wrapping, several workbench views suffer from layout congestion and visual inconsistency:
1. **Missing Dark Mode Neumorphism**: In Dark Mode, cards are darker (20% luminance) than the page canvas (26%), causing panels to appear sunken rather than elevated. Furthermore, the specular rim highlight is virtually invisible (8% opacity) and ambient drop shadows are missing, resulting in flat, wireframed 2D boxes that contrast poorly with Light Mode's rich tactile depth.
2. **Global Target Power Confined to Debugger Tab**: The Target Power Rail (`VTarget`) and live INA180 current sensor are hidden in a large card inside the Debugger view, forcing engineers to navigate away from the Logic Analyzer or USB Adapters just to verify voltage or cycle power.
3. **Overcrowded Debugger Tab**: The bulky power card consumes vertical space needed for memory inspection, register views, and target firmware flashing.
4. **Adapter View Overload**: All 6 hardware bridges (UART, I2C, I3C, SPI, CAN, Industrial) are rendered simultaneously in a massive grid with 6 competing logs, creating visual chaos when users work with one protocol at a time.
5. **Active Bench Observation Bar Congestion**: The waveform toolbar squeezes over 20 controls and 6 dividers into a single row, while the bottom Stimulus Deck (which currently only contains the Signal Generator) consumes 35% of vertical space that belongs to waveform inspection.

This change unifies tactile neumorphic depth across both Dark and Light themes, moves Target Power to a live top-bar toggle pill, restructures the Debugger view into a clean 2-column layout, transforms USB Protocol Adapters into an elegant Master-Detail interface, and streamlines the Active Bench toolbar and Signal Generator drawer.

## What Changes

- **Unified Neumorphic Depth System (Dark & Light)**:
  - Invert dark mode luminance hierarchy: set page canvas to `oklch(18% 0 0)` and card surfaces to `oklch(24% 0 0)` so panels are naturally elevated above the desk.
  - Implement a shared dual-shadow formula: ambient diffuse drop shadow on the bottom-right + crisp specular rim catch highlight (`inset 0 1px 0 rgba(255, 255, 255, ...)`) on the top-left edge in both Dark and Light themes.
  - Soften border tokens from harsh wireframes into translucent boundary creases.
- **Top-Bar Target Power Pill**:
  - Relocate VTarget power control and INA180 current sensing from the Debugger card into a compact live toggle button in the top application header (`[ 3.31V · 14mA ]`).
  - Single-click action directly toggles power rail ON / OFF.
  - Status indicator: vibrant emerald green with soft pulse when rail is enabled; muted gray when in standby/safe state; amber/red flash on overcurrent fault.
- **Streamlined Two-Column Debugger View**:
  - Remove the dedicated power card from `#view-target-station`.
  - Arrange the view into a balanced two-column layout:
    - **Left Column (~400px)**: Universal Target Flasher (Dropzone, chip architecture selection, base address, flash progress bar, and log).
    - **Right Column (`flex: 1`)**: Target SWD / CMSIS-DAP Debugger (target detection badge, execution control toolbar, core registers grid, and live memory viewer).
- **Master-Detail USB Protocol Adapters View**:
  - Restructure `#view-adapters` from a 6-card bento grid into a Master-Detail layout.
  - **Left Master List (~280px)**: Glanceable real-time monitor for all 6 bridges (UART, I2C, I3C, SPI, CAN, Industrial), showing live status dots `●`, baud/frequency, and pin mappings.
  - **Right Detail Pane (`flex: 1`)**: Full-width workspace for the selected protocol with transaction controls, quick preset chips (`Scan Bus`, `Read EEPROM`, `Ping OBD-II`), and high-visibility terminal log.
  - **Persistent Per-Protocol Logs**: Maintain independent log containers in the DOM so switching protocols never drops streaming packets or clears history.
  - **Global Auto-Sniff Toggle**: Header-level switch automatically arms the Logic Analyzer to capture and decode transmitted bus packets.
  - **Smart Default Protocol**: Defaults to UART Bridge, but automatically selects the protocol configured by the active hardware profile.
- **Streamlined Active Bench UI/UX**:
  - Consolidate the 20-item observation toolbar into 3 clear functional clusters:
    - **Capture Cluster**: `[ ▶ RUN / STOP ]` + `Rate: [ 10 MS/s ▾ ]` + `Trigger: [ D0 ⤊ ▾ ]`.
    - **Analysis Cluster**: `[ ☍ Decode ▾ ]` + `[ ⇪ Export ▾ ]` + `Auto-Sniff`.
    - **Hardware Bank Info**: `Bank: GP20–GP27 (8 Ch)`.
  - Move Zoom controls (`+`, `−`, `Fit`) into a sleek floating widget overlay inside the waveform canvas corner.
  - Transform the bottom Stimulus Deck into a **Collapsible Bottom Drawer**:
    - Minimized by default as a compact footer bar (`[ Signal Generator: GP12 @ 12 kHz (50% PWM) ▾ ]`).
    - Expands upward on click to adjust frequency, duty cycle, and pin, freeing up 90%+ of vertical height for waveform analysis.

## Capabilities

### Modified Capabilities
- `bench-theme-engine`: Aligns Dark and Light mode luminance hierarchies; introduces shared dual-shadow neumorphic tokens (ambient drop shadow + specular rim highlight).
- `webui-control-surface`: Adds the live Target Power toggle pill with INA180 telemetry to the top application header.
- `target-station`: Reorganizes the Debugger view into a two-column Flasher and SWD Debugger layout, removing the redundant power card.
- `flexible-adapter-routing`: Converts the 6-adapter grid into a Master-Detail interface with persistent per-protocol logs and quick preset chips.
- `unified-active-bench`: Streamlines the waveform observation toolbar into clustered controls, moves zoom to a canvas floating overlay, and converts the Signal Generator into a collapsible footer drawer.

## Impact

- **Affected Files**:
  - `firmware/html/index.html` (top-bar power pill markup, 2-column debugger layout, master-detail adapters markup, clustered bench toolbar, collapsible signal generator footer)
  - `firmware/html/style.css` (neumorphic depth tokens, top-bar power pill styling, master-detail adapter styles, clustered toolbar layout, floating canvas zoom overlay, collapsible drawer transitions)
  - `firmware/html/app.js` (top-bar power toggle handler and telemetry update loop, adapter master-detail protocol switcher with profile-aware default, active bench clustered event listeners, signal generator drawer toggle)
- **Zero Flash/Firmware C Regressions**: Strictly frontend HTML/CSS/JS assets; zero changes to C microcontroller drivers.
