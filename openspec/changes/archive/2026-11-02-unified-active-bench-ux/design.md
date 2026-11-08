# Unified Active Bench UX & Navigation Technical Design

## Context

The AgentLA WebUI provides high-performance logic analysis (100 MS/s), analog scope, 14+ protocol decoders, multi-protocol bus masters (I2C, SPI, UART, CAN, Modbus, 1-Wire, LIN), pattern generation, and target programming/debugging. Previously, these tools were fragmented across more than 11 navigation views, requiring users to repeatedly toggle views while debugging embedded circuits. 

This design establishes a unified workbench architecture that integrates waveform observation and bus stimulus into a responsive, cohesive surface.

## Goals / Non-Goals

**Goals:**
- Provide a clear, two-mode primary header navigation: `[ Active Bench ]` and `[ Target Station ]`.
- Unify observation (Logic Analyzer canvas, Analog Scope, Protocol Decoders) and interaction (Bus Masters, Stimulus REPL, Pattern Gen, Pin Toggler) in the Active Bench.
- Implement responsive dual-mode layout:
  - Desktop ($\ge 1200\text{px}$): Side-by-side layout with canvas left (65%) and stimulus deck right (35%).
  - Compact / Mobile (<1200px): Stacked split-deck layout with canvas top (60%) and collapsible stimulus deck bottom (40%).
- Implement "Send & Sniff" synchronization: transmitting bus transactions auto-arms capture and centers the canvas on the resulting packet waveform, with a toolbar toggle `[x] Auto-Sniff on Transmit`.
- Provide a slide-out Hardware Pin Drawer for GPIO multiplexing and routing, accessible via top bar or `P` hotkey.
- Maintain lightweight, zero-dependency vanilla ES6/CSS architecture to preserve sub-100KB footprint in firmware flash (`pico_fsdata.inc`).

**Non-Goals:**
- Rewriting backend capture or decoder firmware (already implemented and tested).
- Introducing heavyweight frontend framework dependencies (React, Vue, Webpack).

## Decisions

### 1. Two-Tier Mode Architecture
- **Primary Header Switch**: A segmented control in the top bar switches between `#view-active-bench` and `#view-target-station`.
- **Active Bench Surface**: Houses the waveform canvas, decoder overlays, time markers, and the docked Stimulus Deck.
- **Target Station Surface**: Houses flash file drag-and-drop, target MCU detection, CMSIS-DAP/SWD registers/memory viewer, and VTarget power rail controls.
- *Alternatives Considered*: Floating draggable modal windows. Rejected due to touch screen complexity and visual clutter on laptops.

### 2. Responsive CSS Grid / Flexbox Layout
- Layout container uses `.active-bench-container`.
- Default (<1200px): `display: flex; flex-direction: column;` with canvas taking remaining vertical space and stimulus deck docked at bottom with collapse chevron.
- Desktop ($\ge 1200\text{px}$): `display: grid; grid-template-columns: minmax(0, 1fr) 420px; gap: 1rem;` placing canvas on the left and stimulus deck on the right.
- Canvas resizing is governed by a native `ResizeObserver` that updates internal canvas coordinate dimensions without resetting horizontal zoom or pan offsets.

### 3. Synchronized "Send & Sniff" Workflow
- Toolbar includes `[x] Auto-Sniff on Transmit` checkbox pill (enabled by default).
- When a user transmits an I2C, SPI, UART, CAN, Modbus, 1-Wire, or LIN packet:
  - If capture is idle: Immediately calls `startCapture()`, marks the transmit timestamp, and waits for packet arrival.
  - When capture data arrives: Automatically scrolls and zooms the canvas to the packet interval, expanding the relevant protocol decoder row.
  - If capture is already streaming: Emits a visual marker line at $t_{\text{transmit}}$ without interrupting the stream.

### 4. Slide-Out Pin Matrix Drawer
- Global pin configuration (`#pin-matrix-drawer`) is anchored as a right-hand slide-out drawer (`transform: translateX(100%)`, animated with CSS transitions).
- Opened via `[ Pins ]` button in the header or pressing `P`.
- Features real-time pin conflict alerts and hardware zone ownership indicators.

## Risks / Trade-offs

- **[Risk]** Canvas redraw lag or distorted aspect ratio during layout transitions between desktop and mobile.
  - **Mitigation**: Hook into `transitionend` and `ResizeObserver` to debounce `redrawCanvas()`.
- **[Risk]** Auto-sniff could overwhelm USB or web worker if rapid consecutive bus packets are queued.
  - **Mitigation**: Throttle auto-sniff capture triggers so subsequent sends within 250ms append transaction markers rather than re-arming hardware capture DMA.
- **[Risk]** Firmware flash footprint growth in `pico_fsdata.inc`.
  - **Mitigation**: All new markup and styles are strictly vanilla, modular, and minified in the build pipeline.
