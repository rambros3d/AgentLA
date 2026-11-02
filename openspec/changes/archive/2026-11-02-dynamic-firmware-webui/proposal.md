## Why

Currently, the AgentLA WebUI hardcodes pin mappings (GP0–GP19) and instrument capabilities directly in JavaScript (`firmware/html/app.js`). As the project expands from standard Raspberry Pi Pico 2 boards (30 GPIOs, unbuffered 3.3V, no differential transceivers) to custom hardware such as BakuProbe (RP2350B, 48 GPIOs, level-shifted programmer/bus zones, onboard CAN-FD and RS-485 transceivers, INA180 target current sense), maintaining separate WebUI forks or hardcoded conditionals creates severe drift, bugs, and maintenance overhead.

This change establishes a single, universal WebUI codebase that dynamically configures its navigation tabs, instrument panels, pin matrix, and interactive vector pinout diagrams at runtime based on a declarative board manifest served directly by the connected firmware over USB RNDIS Ethernet.

## What Changes

- **Self-Describing Firmware Assets**: Firmware embeds its active board package (`boards/<board_id>/board.json`, `logo.svg`, `favicon.svg`, `pinout.svg`) into flash via lwIP `httpd` static fsdata, serving `/board.json` directly with zero C runtime memory overhead.
- **Single Source of Truth (SSOT) Build Pipeline**: A Python generator invoked automatically by CMake generates C header constants (`board_config.h`) and stages web assets from `boards/<board_id>/board.json` before compilation.
- **Dynamic Feature Registry**: WebUI navigation tabs and tool views (CAN-FD, RS-485, Power Monitoring, Logic Analyzer channels, Oscilloscope ADCs) mount conditionally based on reported board capabilities.
- **Dynamic Hardware Pin Matrix**: Replaces hardcoded pin lists in `app.js` with dynamic rendering driven by board zone definitions (`gpio_range`, fixed roles, level shifting, connectors).
- **Interactive Inlined SVG Pinout**: Inlines the board's `pinout.svg` into the DOM and establishes bidirectional event binding between SVG pin elements (`<g id="pin-gpX">`) and the Pin Matrix table.
- **Buffer & Caching Resilience**: Increases lwIP POST buffer to 2048 bytes in `network_stack.c` and enforces versioned query strings (`/board.json?v=<version>`) to prevent cross-board browser cache pollution.

## Capabilities

### New Capabilities
- `dynamic-board-manifest`: Declarative board specification schema (`board.json`), automated CMake pre-build code/asset generation, and static HTTP serving via lwIP fsdata.
- `interactive-pinout-svg`: Dynamic vector pinout diagram loading, inlining into the DOM, and reactive bidirectional synchronization between SVG pin pads and UI instrument configurations.

### Modified Capabilities
- `hardware-pin-drawer`: Requirements change from static pin lists to dynamically populated pin matrix rows, voltage rail indicators, and zone grouping parsed from `/board.json`.
- `webui-control-surface`: Requirements change so instrument tabs (CAN-FD, RS-485, Target Power, Scope channels) dynamically show or hide depending on the hardware capabilities reported by the firmware.

## Impact

- **Firmware Flash & lwIP**: `network_stack.c` increases `httpd_post_body` from 768 to 2048 bytes; CMake dynamically selects `boards/${BOARD_ID}` files for `pico_set_lwip_httpd_content`.
- **WebUI Client**: `firmware/html/app.js` refactored to eliminate hardcoded pin tables and implement capability-based feature mounting.
- **Build System**: New build tool `tools/gen_board_assets.py` integrated into CMake build pipeline.
- **Hardware Packages**: Standardized `boards/pico2/`, `boards/bakuprobe/`, and `boards/templates/` directory structures.
