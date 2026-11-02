# Unified Active Bench UX & Navigation Redesign

## Why

The current AgentLA WebUI presents excessive cognitive load with over 11 distinct sidebar/tab navigation items, forcing hardware engineers to constantly switch between separate "Observe" (Logic Analyzer, Oscilloscope, Protocol Decoders) and "Interact" (Bus Masters, Stimulus REPL, Signal Generator) surfaces. Testing hardware protocols requires an integrated feedback loop where issuing a stimulus transaction immediately reflects in the captured waveform and decoded packets. 

By unifying Observe and Interact into a cohesive **Active Bench** (with responsive side-by-side or split-deck views and automatic "Send & Sniff" capture triggering) and simplifying global navigation into two primary modes—**Active Bench** and **Target Station**—the user experience transforms from an overwhelming instrumentation dashboard into a streamlined, intuitive hardware workbench.

## What Changes

- **Streamlined Top-Level Navigation**: Replace the multi-tab navigation with two prominent primary modes:
  - `[ ⚡ Active Bench ]`: Unified live observation (logic capture, scope, protocol decoders) and stimulus interaction (bus masters, pattern gen, GPIO toggler).
  - `[ 🎯 Target Station ]`: Dedicated flashing, target detection, CMSIS-DAP/SWD debugging, and target power management.
- **Unified Active Bench Layout**:
  - Responsive Side-by-Side layout on desktop screens ($\ge 1200\text{px}$): Waveform canvas on left (65%), docked Stimulus Deck on right (35%).
  - Responsive Split-Deck layout on smaller/mobile displays (<1200px): Waveform canvas top (60%), collapsible Stimulus Deck bottom (40%).
- **Interactive "Send & Sniff" Synchronization**:
  - When transmitting any transaction on a Bus Master (I2C, SPI, UART, CAN, Modbus, 1-Wire, LIN), an automatic capture sequence triggers immediately (if idle), auto-zooming and centering the timeline on the transmitted transaction.
  - Includes a visible `[x] Auto-Sniff on Transmit` toggle pill in the bench toolbar so users can toggle between auto and manual triggering at will.
- **Slide-out Pin Matrix Drawer**:
  - Global hardware pin configuration and transceiver routing is moved into a clean slide-out drawer accessible via a top-bar `[ 🔌 Pins ]` button or hotkey `P`, featuring live pin conflict badges.
- **Demo & Firmware Synchronization**:
  - Ensure identical responsive UX and styling across both the hosted web portal (`web/demo/`) and the embedded web server (`firmware/html/`).

## Capabilities

### New Capabilities
- `unified-active-bench`: Unified observation canvas and stimulus deck with responsive desktop side-by-side (65/35) and mobile split-deck layouts, plus "Send & Sniff" synchronized bus triggering.
- `target-station`: Consolidates microcontroller flashing, SWD/CMSIS-DAP debug inspection, automatic chip detection, and target VTarget power switching into a dedicated station.
- `hardware-pin-drawer`: Slide-out pin matrix drawer providing interactive GPIO assignment, internal bus routing, and conflict indication without cluttering active workbench operations.

### Modified Capabilities
- `webui-control-surface`: Replaces the 11-panel tab navigation layout with two primary mode switches (`Active Bench` vs `Target Station`) and the slide-out pin drawer.

## Impact

- **Web Frontend**:
  - `firmware/html/index.html` & `web/demo/index.html`: Refactored layout structure, top bar, docked stimulus deck, and pin drawer.
  - `firmware/html/style.css` & `web/demo/style.css`: Responsive CSS grid/flex styles, drawer transitions, and Send & Sniff visual indicators.
  - `firmware/html/app.js` & `web/demo/app.js`: Unified state management, mode switching, Send & Sniff auto-trigger hooks, and pin drawer event listeners.
- **Embedded Firmware**:
  - Regenerated `generated/pico_fsdata.inc` to serve the modernized WebUI from Pico 2 / BakuProbe flash without exceeding SRAM/Flash limits.
