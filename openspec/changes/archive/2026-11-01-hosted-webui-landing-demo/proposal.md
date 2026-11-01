## Why

AgentLA features a touch and desktop adaptive WebUI embedded directly in the RP2350 firmware. However, users wanting to evaluate the tool before purchasing or flashing hardware cannot explore the WebUI without a running physical probe. Furthermore, users often need a central entry point to search for connected local hardware or explore documentation and hardware specs.

Hosting a demo and landing page directly on GitHub Pages or Cloudflare Pages provides an immediate, zero-install showcase for prospective users. Crucially, this must be accomplished without adding any bloated demo data or simulation assets to the microcontroller firmware flash image (`pico_fsdata.inc`).

## What Changes

- Create a dedicated, standalone `web/` directory that hosts the public portal independently of `firmware/html/`.
- Build a **Marketing & Discovery Splash Page** (`web/index.html`) featuring:
  - Product overview and hardware capabilities (100 MS/s Logic Analyzer, Multi-core Flasher, Bus Masters, Dynamic Pin Matrix).
  - "Search / Open Local Device" feature that tests local device reachability (`http://192.168.7.1` / `http://agentla.local`) and offers 1-click launch.
  - Optional WebSerial direct connection button (`navigator.serial`) for browser-to-USB communication over HTTPS.
  - 1-tap "Launch Interactive Demo" button leading to the standalone WebUI simulator.
- Build an **In-Browser Hardware Simulator & Demo WebUI** (`web/demo/`):
  - Shares the responsive, touch-friendly WebUI shell without polluting the firmware tree.
  - Implements a pure client-side `mock-bridge.js` providing synthetic 100 MS/s logic captures (I2C sensor read, SPI flash boot, UART greeting), simulated telemetry (150 MHz sys clock, 1.1V vreg), and an interactive 30-pin dynamic matrix playground (GP0–9 fixed programmer block vs GP10–29 flexible dynamic claims).
- Add automated GitHub Actions deployment workflow (`.github/workflows/deploy-pages.yml`) to publish `web/` to GitHub Pages automatically.
- Keep `firmware/html/` strictly minimal with zero mock datasets or simulation scripts compiled into the microcontroller binary.

## Capabilities

### New Capabilities
- `hosted-marketing-splash`: Landing page with product overview, interactive pinout reference, and local subnet probe discovery / WebSerial connection options.
- `browser-hardware-simulator`: Pure client-side hardware simulator generating realistic multi-channel waveforms, telemetry streams, and dynamic pin claim responses.
- `static-portal-deployment`: Isolated `web/` portal package and automated GitHub Pages deployment workflow ensuring zero impact on RP2350 firmware flash size.

### Modified Capabilities
<!-- None: Embedded firmware behavior and specs remain unchanged -->

## Impact

- **Firmware Binary**: Zero impact. No new files added to `firmware/html/`, and `pico_fsdata.inc` remains unbloated.
- **New Directory**: `web/` containing static HTML, CSS, JavaScript, and mock waveform data.
- **CI/CD**: Added `.github/workflows/deploy-pages.yml` for automated GitHub Pages static site deployment.
