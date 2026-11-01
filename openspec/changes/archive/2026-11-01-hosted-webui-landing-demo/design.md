## Context

AgentLA's embedded WebUI provides a dual-theme, dual-slot workspace running on an RP2350 microcontroller. To provide a public online demo and discoverability portal without bloating the microcontoller's flash memory, we must isolate all hosted, demo, and marketing code into a dedicated directory (`web/`) deployed independently via GitHub Pages.

## Goals / Non-Goals

**Goals:**
- Provide a clean, modern landing page with product introduction, hardware specifications, and an interactive 30-pin allocation map.
- Provide a 1-click device discovery link for users whose hardware is already running at `http://192.168.7.1` or `http://agentla.local`.
- Provide a fully functional in-browser WebUI simulation mode (`web/demo/`) using synthetic waveform streams, simulated pin registry, and live telemetry without requiring physical hardware.
- Support optional WebSerial USB connection directly from modern desktop/mobile browsers.
- Ensure 0 bytes of overhead added to the embedded firmware binary (`pico_fsdata.inc`).
- Automate static hosting deployment via GitHub Actions.

**Non-Goals:**
- Emulating complex physical electrical faults or signal noise (simulation is purely digital protocol frames).
- Running an active backend server in the cloud (hosted site is 100% static client-side JavaScript, CSS, and HTML).

## Decisions

### 1. Isolated `web/` Directory Architecture
- **Decision**: Keep the hosted portal completely in `web/` (`web/index.html` for landing splash, `web/demo/` for simulator).
- **Rationale**: Isolating `web/` from `firmware/html/` ensures `makefsdata` and `CMakeLists.txt` continue packaging only the lightweight embedded assets into the RP2350 flash image.
- **Alternative considered**: Conditional build tags in `firmware/html/app.js`. Rejected because embedding mock waveform datasets would inflate `pico_fsdata.inc` by dozens of kilobytes.

### 2. Client-Side Synthetic Hardware Bridge (`mock-bridge.js`)
- **Decision**: Implement a standalone `mock-bridge.js` loaded exclusively in `web/demo/index.html`.
- **Rationale**: Replaces fetch/SSE endpoints (`/api/command`, `/events`) with an event-driven virtual state machine. Generates SPI, I2C, and UART packet bursts at 100 MS/s sample clock, simulated telemetry, and dynamic pin claim responses matching the real firmware protocol.

### 3. Local Subnet Discovery via Fetch / Favicon Probe
- **Decision**: The landing page issues a non-blocking asynchronous fetch/ping to `http://192.168.7.1/ok.json` (with timeout and error fallback).
- **Rationale**: Detects whether the user is browsing on a computer connected to AgentLA over USB Ethernet or Wi-Fi, dynamically updating the status pill to "Probe Detected" with an instant jump button.

### 4. WebSerial API Integration
- **Decision**: Implement `navigator.serial` in `web/demo/` for direct browser-to-USB control.
- **Rationale**: Bypasses HTTPS/HTTP mixed content restrictions and enables direct hardware interaction from the hosted web app.

## Risks / Trade-offs

- **[Mixed Content Security]** Modern browsers prevent HTTPS pages (`https://...github.io`) from making silent background HTTP AJAX requests to local IPs (`http://192.168.7.1`).
  - *Mitigation*: The landing page provides direct top-level navigation links (`target="_blank"` or redirect) and uses WebSerial for in-page direct hardware communication.
- **[Waveform Realism]** Simulated waveforms must accurately showcase the Logic Analyzer's features.
  - *Mitigation*: Generate authentic digital trace patterns for I2C (START, 7-bit addr, ACK, data, STOP) and SPI (CS asserted, SCK clock bursts, MOSI/MISO byte pairs).
