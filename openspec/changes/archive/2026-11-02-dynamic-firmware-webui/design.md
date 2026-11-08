## Context

AgentLA supports multiple physical hardware targets based on the Raspberry Pi RP2350 microcontrollers:
1. **Pico 2**: Standard RP2350A (QFN-60, 30 GPIOs, 4 ADCs, direct 3.3V LVCMOS, no industrial transceivers).
2. **BakuProbe**: Custom multi-zone workstation PCB using RP2350B (QFN-80, 48 GPIOs, 8 ADCs, level translation, CAN-FD TCAN334, RS-485 SN65HVD3082, INA180 target current sense).

Currently, `firmware/html/app.js` hardcodes pin arrays (GP0–GP19) and static layout assumptions. This design establishes a dynamic, self-describing hardware architecture where the WebUI client loads board identity and capabilities from the firmware over USB RNDIS Ethernet at runtime.

## Goals / Non-Goals

**Goals:**
- Provide a single unified WebUI application shared across all hardware profiles without code forks.
- Maintain a single source of truth (SSOT) per board in `boards/<board_id>/board.json`.
- Automatically generate low-level C firmware headers and bundle static web assets via CMake before build.
- Embed `/board.json`, `/logo.svg`, and `/pinout.svg` into flash via lwIP `httpd` with zero runtime RAM consumption.
- Dynamically adapt WebUI navigation tabs, tool panels, pin matrix rows, and interactive vector pinout diagrams.
- Protect against cross-board browser cache corruption and POST payload truncation.

**Non-Goals:**
- WebSerial / WebUSB transport support (explicitly excluded; communication is strictly USB RNDIS Ethernet over HTTP / SSE).
- Runtime reconfiguration of PCB hardware traces or dynamic board hot-swapping without re-enumeration.

## Decisions

### 1. Board Bundle Directory Layout (`boards/<board_id>/`)
- **Decision**: Organize hardware configurations into self-contained board folders (`boards/pico2/`, `boards/bakuprobe/`, `boards/templates/`) containing `board.json`, `logo.svg`, `favicon.svg`, `pinout.svg`, and `README.md`.
- **Rationale**: Isolates visual branding and vector graphics from generic WebUI application code. Adding new boards requires only adding a new folder without editing global JS or CSS.
- **Alternatives Considered**: Flat `boards/` directory with `pico2.json` and `pico2_pinout.svg` (rejected due to clutter and weak 1:1 asset coupling).

### 2. CMake Automated Code Generation (`tools/gen_board_assets.py`)
- **Decision**: Integrate a pre-build Python script into CMake that compiles `board.json` into:
  1. `firmware/src/generated/board_config.h` (pin constant `#defines`, peripheral capability flags)
  2. Staged web directory containing minified `board.json` and board SVGs alongside shared HTML/JS assets.
- **Rationale**: Guarantees zero drift between C drivers and the WebUI. If a pin mapping changes in `board.json`, both firmware and UI update simultaneously on build.
- **Alternatives Considered**: Manually maintained C headers (rejected due to risk of divergence with JSON manifests).

### 3. Flash-Resident lwIP fsdata Serving
- **Decision**: Bundle `board.json` directly into lwIP's compile-time fsdata blob via `pico_set_lwip_httpd_content`.
- **Rationale**: The RP2350 has ample flash (4MB on Pico 2, 16MB on BakuProbe). Serving static assets from flash costs 0 bytes of SRAM and requires no runtime C endpoint handlers in `webui_command_bridge.c`.
- **Alternatives Considered**: Dynamic C-based REST endpoint `/api/board` returning stringified JSON (rejected: wastes SRAM buffers and duplicate memory).

### 4. Modular Client Feature Registry & External Breakout Mode
- **Decision**: Refactor `firmware/html/app.js` around a declarative Feature Registry:
  ```javascript
  const FEATURE_REGISTRY = [
    { id: 'can', isEnabled: (caps) => !!caps.has_hardware_can, mount: () => ... },
    { id: 'rs485', isEnabled: (caps) => !!caps.has_hardware_rs485, mount: () => ... },
    { id: 'power', isEnabled: (caps) => !!caps.target_current_sense, mount: () => ... }
 ];
  ```
  When hardware-dedicated transceivers are absent (such as CAN-FD or RS-485 on a standard Pico 2), the cards in the **Adapters** tab switch into **External Breakout Mode** instead of being hidden. The UI provides explicit breakout pinout diagrams (e.g., GP18 TX, GP19 RX, GP22 Term) and wiring instructions for connecting standard external transceiver modules.

### 5. Dual-Tier Pin Matrix Architecture & Inlined Vector Pinout
- **Decision**: Embed the board's interactive vector pinout diagram (`pinout.svg`) directly inside the dedicated full-screen **Pin Matrix** view (`#view-pin-matrix`), arranged side-by-side with the pin routing matrix.
- **Partitioning Model**:
  1. **Fixed Function Groups**: Hardware-dedicated peripherals (Target Programmer SWD/JTAG, CAN/RS-485 Differential Transceivers, Analog ADC Scope inputs, and Target Power sensing) are distinctly grouped with their physical connector pinouts, voltage level-shifting ($1.2\,\text{V} - 5.0\,\text{V}$), and reference rails.
  2. **Unified Flexible Digital Pool**: Unconstrained digital GPIOs (PIO state machines, digital logic analyzer capture channels, signal generator outputs, and assignable bus masters) are presented in a unified flexible pool, reflecting their software-definable nature without artificial zone restrictions.
- **Interactive Inlining**: The WebUI asynchronously fetches `/pinout.svg` and injects its root `<svg>` directly into the DOM container. Hovering or clicking pins provides bidirectional synchronization between SVG pin elements (`<g id="pin-gpX">`) and the Pin Matrix table.

### 6. Buffer Expansion & Cache Invalidation
- **Decision**: Expand `httpd_post_body` from 768 to 2048 bytes in `network_stack.c`. Fetch `/board.json?v=<version>` with cache-busting headers.
- **Rationale**: Prevents buffer truncation for multi-channel analyzer configurations and ensures browsers do not retain cached Pico 2 assets when connecting to a BakuProbe on the same IP.

## Risks / Trade-offs

- **[Risk] lwIP httpd SVG MIME Type Mappings** → Default lwIP might serve `.svg` as `text/plain` or `.bin`.  
  *Mitigation*: Verify and declare `.svg` $\rightarrow$ `image/svg+xml` and `.json` $\rightarrow$ `application/json` in lwIP fs data generation settings.
- **[Risk] Large Vector Pinouts Slowing Down Page Load** → Complex PCB graphics could bloat flash and slow down rendering.  
  *Mitigation*: Maintain clean, simplified vector geometry for pinout SVGs (< 25 KB uncompressed, < 8 KB gzipped).
- **[Risk] Pin Matrix DOM Rendering Overhead** → BakuProbe has 48 GPIOs across 5 zones.  
  *Mitigation*: Render pin rows using efficient DocumentFragment batches during drawer initialization.
