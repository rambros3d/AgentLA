# Proposal: Full Hardware REST Suite

## Why

While AgentLA introduced a dedicated REST API (`/api/v1/`) for LittleFS app installation, settings, and themes, the actual hardware instrumentation engines (hardware profiles, 30-channel pin ownership/GPIO, target programming, logic analysis, and peripheral bus bridges) remain locked behind legacy CLI command strings on the `/mcp` JSON-RPC tunnel. Additionally, the web app launcher directly attempts to call `/api/profiles` and `/api/profiles/apply`, which fail with 404s on physical hardware.

Providing a full hardware REST suite modernizes the probe's HTTP surface, eliminates brittle stdout string parsing in the Web UI, allows LittleFS community apps to directly control physical hardware using standard `fetch()` calls, and establishes streaming `.vcd` and raw binary downloads using zero-RAM generator streaming.

---

## What Changes

This change introduces a phased REST API across three delivery milestones:

### Phase 1: Profiles, Pins & Probe Telemetry (Foundations & Bug Fixes)
- **Fix Broken Profile UI**: Implement `GET /api/v1/profiles`, `POST /api/v1/profiles/apply`, `POST /api/v1/profiles/boot`, and `POST /api/v1/profiles/save`, with legacy route aliases for `/api/profiles`.
- **Full 30-Pin Inspection**: Expand `GET /api/v1/pins` from an empty stub into a complete 30-channel GPIO map returning domain ownership (`PROGRAMMER`, `ADAPTER`, `ANALYZER`), mode, pull configuration, and logic levels.
- **Single-Pin Control**: Add `GET /api/v1/pins/{gpio}`, `POST /api/v1/pins/{gpio}/mode`, and `POST /api/v1/pins/{gpio}/write`.
- **Probe Telemetry & Control**: Add `GET /api/v1/device/telemetry` (VTARGET, temperature, SRAM heap), `POST /api/v1/device/power`, `POST /api/v1/device/reboot`, and `POST /api/v1/device/bootsel`.
- **Memory Optimization**: Resize `rest_response_buffer` to 2048 bytes with compact JSON schema to ensure all control responses fit cleanly within a single TCP segment.

### Phase 2: Peripheral Bus Bridges & Addressable LED Driver (`/api/v1/bus`)
- **I2C Bridge**: `POST /api/v1/bus/i2c/scan` returning detected 7-bit addresses; `POST /api/v1/bus/i2c/transfer` for atomic write/read operations.
- **SPI Bridge**: `POST /api/v1/bus/spi/transfer` with configurable mode (0–3), clock speed, chip-select control, and full-duplex hex payloads.
- **UART Bridge**: `POST /api/v1/bus/uart/write` and `GET /api/v1/bus/uart/read` with configurable baud rates.
- **CAN Bus Bridge**: `POST /api/v1/bus/can/tx` and `GET /api/v1/bus/can/rx` for standard and extended automotive CAN frames.
- **Universal Addressable LED Driver**: `POST /api/v1/bus/led/transmit` driving up to 256 pixels across both 1-wire PWM/NRZ (WS2812B, SK6812 RGB & 32-bit RGBW, WS2811 800k/400k, WS2813) via `neopixel.pio` and 2-wire clocked SPI (APA102/DotStar, SK9822).

### Phase 3: Core Flashing & Logic Analyzer Instruments (`/api/v1/programmer`, `/api/v1/analyzer`)
- **Target Programmer**:
  - `GET /api/v1/programmer/target`: Autonomous target MCU identification (STM32, PY32, CH32, RP2040/RP2350, nRF).
  - `POST /api/v1/programmer/reset`: Hardware reset via target `nRST` and `BOOT0` control.
  - `POST /api/v1/programmer/erase`: Mass or sector erase.
  - `POST /api/v1/programmer/flash`: Direct binary or hex flashing with algorithm runner and verify.
  - `GET /api/v1/programmer/read`: Chunked flash/SRAM inspection stream.
- **Logic Analyzer & VCD Streaming**:
  - `POST /api/v1/analyzer/arm`: Configure sample rate, channels (GPIO 20–29), sample count, and triggers.
  - `GET /api/v1/analyzer/status`: Polling capture state (`idle`, `armed`, `capturing`, `done`).
  - `POST /api/v1/analyzer/stop`: Abort capture.
  - `GET /api/v1/analyzer/vcd`: Zero-RAM generator streaming via `fs_read_custom` producing Sigrok/PulseView `.vcd` files directly from `captureBuffer`.
  - `GET /api/v1/analyzer/data`: Stream raw or RLE-compressed sample transitions.

---

## Capabilities

### New Capabilities
- `rest-hardware-profiles`: Complete REST endpoints for slot listing, applying, saving, and boot slot selection under `/api/v1/profiles` and `/api/profiles`.
- `rest-gpio-pins`: Full 30-pin state table, individual pin direction/pull/value manipulation, and pin arbitration under `/api/v1/pins`.
- `rest-bus-bridges`: Structured JSON-based bus scanning, reading, writing, and transfers for I2C, SPI, UART, and CAN under `/api/v1/bus`.
- `rest-addressable-led`: Universal dual-family addressable LED driver supporting single-wire PWM (WS2812B/SK6812/WS2811) and 2-wire SPI (APA102/DotStar) for up to 256 pixels under `/api/v1/bus/led`.
- `rest-target-programmer`: Target MCU auto-detection, reset, sector/mass erase, binary flashing, and flash read operations under `/api/v1/programmer`.
- `rest-logic-analyzer`: Capture arming, state polling, sample retrieval, and direct Sigrok `.vcd` export streaming under `/api/v1/analyzer`.
- `rest-device-telemetry`: Voltage rail measurement, target power switching, MCU temperature, heap metrics, reboot, and bootloader entry under `/api/v1/device`.

### Modified Capabilities
- `url-routing-engine`: Route expansion to dispatch `/api/v1/profiles`, `/api/v1/pins`, `/api/v1/bus`, `/api/v1/programmer`, `/api/v1/analyzer`, `/api/v1/device`, and legacy `/api/profiles` aliases through `rest_api_handle`.

---

## Impact

- **Firmware Engine**:
  - `firmware/pio/neopixel.pio`: Hardware-timed single-wire NRZ driver state machine with 800k/400k clock divider scaling.
  - `firmware/src/rest_api.c` & `rest_api.h`: Implements the new REST endpoints and tiered hybrid dispatch.
  - `firmware/src/app_routing.c`: Extends URL matching and registers `fs_read_custom` stream generators for VCD and flash reading.
  - `firmware/src/network_stack.c`: Supports POST payload interception and multi-part data routing.
- **Web Frontend**:
  - `firmware/html/apps.html`: Fixes the profile menu to successfully load and apply hardware profile slots without failing.
  - `firmware/html/app.js`: Enables workbench instruments to migrate from stdout regex parsing to clean REST JSON responses.
- **Compatibility**:
  - Preserves full backwards compatibility for `/mcp` JSON-RPC and console CLI.
  - Zero RAM penalty on RP2040/RP2350 due to the Tiered Hybrid streaming design.
