## Context

AgentLA's primary instrument view has been titled "Active Bench", housing both the 100 MS/s digital logic analyzer and the 500 kS/s analog oscilloscope. Historically, users switched between digital and analog viewing using a mutually exclusive toggle (`[ Logic ] [ Scope ]`). In real-world mixed-signal debugging—such as watching a 3.3V power drop or sensor analog voltage step occurring during an I2C or SPI transaction—switching views loses chronological correlation and temporal context.

Additionally:
- Pin assignments are opaque and require navigating away to the Pin Matrix or scanning channel buttons.
- Digital and analog triggering were conflated with a generic voltage threshold, despite RP2350 digital inputs using fixed CMOS Schmitt-trigger silicon ($V_{IL} \approx 0.8\text{V}$, $V_{IH} \approx 2.0\text{V}$).
- The observation toolbar suffered from horizontal overcrowding (20 controls in a single line).
- The protocol decoder button opened an awkward popup over the waveforms.
- The bottom Stimulus Deck (Signal Generator) crowded the vertical canvas space.

## Goals / Non-Goals

**Goals:**
- Rename "Active Bench" to **"Scope"** across desktop sidebar (-90° rotated word "SCOPE"), mobile bottom dock, and view titles.
- Implement a **Unified MSO (Mixed-Signal Oscilloscope) Window** with:
  - Top subwindow: **Digital Scope (Logic Analyzer)** in multi-lane stacked format (Sigrok / PulseView style) with in-lane frequency and duty measurements.
  - Tactile draggable horizontal splitter divider (`.mso-window-splitter`) with height preference memory.
  - Bottom subwindow: **Analog Scope (Oscilloscope)** where all active analog channels share an **8 × 10 division graticule** with calibrated `Volts/div` and `Time/div`.
  - Dynamic visibility: The Analog Scope renders **only** when at least one analog pin is active; otherwise Digital takes 100% of the window.
- Surface all configurable probe pins (`GP10`–`GP29`) across a top pin bar with color-coded status dots (`●` digital trace color, `◈` ADC tag for GP26–GP29, `○` unconfigured).
- Create an **Interactive Pin Configuration Modal** (`#modal-pin-config`) featuring an **18-color curated swatch grid** (no color wheels), custom channel labels, and hardware-accurate trigger rules (edge/state for digital, voltage comparator for analog).
- Position the **Protocol Decoder Bar directly above the digital scope lanes**, with in-canvas packet bubbles, hover tooltips, and a collapsible Decoded Packet Table.
- Streamline toolbar controls into 3 clusters (Capture, Analysis, Bank Info), move zoom to a floating canvas overlay, and transform the Signal Generator into a collapsible footer drawer.

**Non-Goals:**
- Redesigning hardware PIO state machines or ADC DMA acquisition logic in C firmware (existing firmware capture engines already supply high-speed digital and analog buffers).
- Implementing analog trigger hardware comparators for digital GPIOs (RP2350 digital pins have fixed CMOS silicon levels).

## Decisions

### 1. Renaming "Active Bench" to "Scope"
- **Rationale**: "Scope" is the universal industry standard term for waveform visualization instruments (Oscilloscopes, Mixed-Signal Scopes, Logic Scopes). It is concise, fits perfectly in the vertical desktop sidebar (-90° rotated word "SCOPE"), and eliminates confusion with the general workbench.
- **Affected Elements**: Desktop vertical sidebar, mobile bottom dock, view header title, and hotkey hints (`Scope (Hotkey: 1)`).

### 2. Unified MSO Window with Draggable Splitter
- **Rationale**: True mixed-signal analysis requires observing digital bus transitions and analog sensor voltages simultaneously on the same timeline.
- **Subwindow Architecture**:
  - **Digital Scope (Top)**: Multi-lane isolated tracks. Each digital channel has its own private lane with zero waveform collision, accompanied by left badge with pin name, custom label, and live metrics (`[12.5kHz · 50%]`).
  - **Draggable Splitter Divider (`.mso-window-splitter`)**: Horizontal divider bar with handle `⋯`. Dragging dynamically recalculates canvas dimensions and triggers redraws. The split ratio is saved to `localStorage['agentla_mso_split']`.
  - **Analog Scope (Bottom)**: Standard 8 × 10 division graticule. All active analog channels (`ADC0`–`ADC3`) are superimposed on the same graticule, allowing immediate visual amplitude and phase comparison. Features calibrated `Volts/div` (0–3.3V) on the left margin and draggable `◀ Trig Level` on the right.
  - **Dynamic Visibility**: If zero analog pins are configured, the Analog subwindow and splitter hide automatically, giving Digital 100% of the viewport.

### 3. Top Pins Bar & 18 Preset Trace Colors Palette
- **Top Pins Bar**: Renders buttons for `GP10` through `GP29`. Status dots reflect state: active digital color, `◈ ADC` tag for GP26–GP29, or muted outline.
- **18 Preset Swatches**: Curated 6 × 3 grid of vibrant, high-contrast colors:
  - Row 1 (Cyans & Blues): `#00E5FF`, `#38BDF8`, `#3B82F6`, `#6366F1`, `#8B5CF6`, `#A855F7`
  - Row 2 (Pinks, Reds & Warm): `#EC4899`, `#F43F5E`, `#EF4444`, `#F97316`, `#F59E0B`, `#EAB308`
  - Row 3 (Greens, Teals & Slate): `#84CC16`, `#22C55E`, `#10B981`, `#14B8A6`, `#06B6D4`, `#94A3B8`
- **Validation**: Only GP26–GP29 allow selecting "Analog Input (Oscilloscope ADC)". Non-ADC pins disable the option with a descriptive notice.

### 4. Hardware-Accurate Trigger Rules
- **Digital Pins**: Only logical edge/state options are presented (`None`, `Rising Edge ⤊`, `Falling Edge ⤋`, `Either Edge ⇅`, `High 1`, `Low 0`). No voltage threshold input is displayed.
- **Analog Pins**: Voltage comparator threshold is displayed (e.g. `1.65 V`), synchronized with the scope's right-margin trigger cursor.

### 5. Integrated Decoder Bar Above Digital Scope
- **Placement**: Directly above the digital scope canvas, serving as its header.
- **In-Canvas Packet Bubbles**: Decoded packets (`START`, `ADDR: 0x50+W`, `ACK`, `DATA: 0x33`, `STOP`) render time-aligned below the waveforms. Hovering shows a rich transaction tooltip; clicking centers and zooms the canvas to that packet.
- **Decoded Packet Table (`[ Packet Table ]`)**: Slide-out drawer listing all captured transactions chronologically with search/filter capabilities.

### 6. Clustered Toolbar & Collapsible Signal Generator Drawer
- **Toolbar Clusters**:
  - **Capture**: `[ ▶ RUN / STOP ]` + `Rate: [ 10 MS/s ▾ ]` + `Trigger: [ D0 ⤊ ▾ ]`.
  - **Analysis**: `[ ☍ Decode ▾ ]` + `[ ⇪ Export ▾ ]` + `Auto-Sniff`.
  - **Hardware Bank Info**: `Bank: GP10–GP17 (8 Ch)`.
- **Canvas Zoom Widget**: Semi-transparent floating overlay (`+`, `−`, `Fit`) in the top-right corner of the canvas.
- **Signal Generator Drawer**: Minimized footer pill (`[ Signal Generator: GP12 @ 12.0 kHz (50% PWM) ▴ ]`) that smoothly expands upward on click, leaving 90%+ vertical height for waveforms when closed.

## Risks / Trade-offs

- **[Risk] High-DPI Canvas Rendering during Splitter Drag**: Dragging the splitter triggers continuous canvas resizing which could cause visual stutter if not throttled.
  - **Mitigation**: Splitter mousemove events use `requestAnimationFrame` debouncing, resizing DOM canvas buffers and redrawing only on animation frames.
- **[Risk] Screen clutter when many digital channels are enabled**: Enabling all 8 digital channels plus 4 analog channels on small laptop screens could compress channel heights.
  - **Mitigation**: The digital scope enforces a minimum lane height of 28px with smooth vertical scroll if channel count exceeds available subwindow height; users can also drag the splitter to adjust balance.
