# Technical Design: Full Hardware REST Suite

## Context

AgentLA integrates a 10-channel logic analyzer, multi-target programmer, peripheral bus bridges, and hardware profile manager on an RP2040/RP2350 microcontroller running lwIP `httpd`. While app management and theme styling have dedicated `/api/v1/` endpoints, core hardware capabilities are only accessible via raw text strings passed through `/mcp`.

This design outlines the architecture for exposing all hardware instruments through structured REST endpoints, using a Tiered Hybrid memory model to respect microcontroller SRAM limitations.

---

## Goals / Non-Goals

### Goals
- Expose Hardware Profiles, 30-channel GPIO state, Peripheral Bus Bridges, Target Programmer, Logic Analyzer, and Probe Telemetry through clean `/api/v1/...` REST endpoints.
- Fix broken profile switching in the Web UI (`apps.html`) by routing both `/api/v1/profiles` and `/api/profiles`.
- Prevent RAM exhaustion on RP2040 (264 KB) and RP2350 (520 KB) when serving large payloads (VCD waveforms, flash dumps) via chunked zero-RAM generator streaming (`fs_read_custom`).
- Preserve complete backwards compatibility with legacy `/mcp` JSON-RPC and console CLI.

### Non-Goals
- Replacing the CMSIS-DAP v1/v2 USB endpoints or CDC UART ports (these remain primary interfaces for host IDEs like PyOCD / OpenOCD).
- Providing client-side protocol decoding in C firmware (protocol decoders run in JavaScript or host Sigrok; firmware streams raw transitions or `.vcd`).

---

## Decisions

### 1. Two-Tier Response Engine (Static JSON vs Generator Streaming)
- **Problem**: Microcontroller SRAM is constrained (only ~20 KB free headroom after `captureBuffer` allocations). Standard JSON stringification of a 400 KB capture buffer or 64 KB flash dump would cause immediate stack/heap overflow.
- **Decision**:
  - **Tier 1 (Static Buffer)**: Expand `rest_response_buffer` in `rest_api.c` from 1536 B to 2048 B. Use compact JSON for single-request/single-response control endpoints (`/api/v1/profiles`, `/api/v1/pins`, `/api/v1/bus/*`, `/api/v1/programmer/target`, `/api/v1/device/*`).
  - **Tier 2 (Zero-RAM Streaming)**: For `/api/v1/analyzer/vcd`, `/api/v1/analyzer/data`, and `/api/v1/programmer/read`, implement dynamic generator callbacks hooked into lwIP's `fs_read_custom()`. Each TCP packet pulls directly from `captureBuffer` or flash memory into the outgoing lwIP pbuf.
- **Alternatives Considered**:
  - *Dynamic malloc/free per request*: Rejected due to heap fragmentation and OOM risks on Cortex-M.
  - *Base64 JSON encoding in RAM*: Rejected because it inflates payload size by 33% and exceeds SRAM capacity.

### 2. URI Hierarchy and Legacy Aliasing
- **Decision**: All new endpoints live under `/api/v1/<domain>/`. To resolve existing frontend calls in `firmware/html/apps.html`:
  - `GET /api/profiles` aliases to `GET /api/v1/profiles`
  - `POST /api/profiles/apply` aliases to `POST /api/v1/profiles/apply`
- **Alternatives Considered**: Modifying only the frontend JavaScript: Rejected because existing deployed web apps and cached browser bundles expect `/api/profiles`. Both the firmware route table and web files should be aligned.

### 3. Compact Key Schema for Pin State (`/api/v1/pins`)
- **Decision**: Format 30 pins using compact JSON keys:
  `{"p":0,"d":"PROG","m":"PIO","v":1,"pu":"none"}`
  This allows all 30 pins to serialize into ~1250 bytes, comfortably fitting into the 2048 B buffer with headers.
- **Alternatives Considered**: Verbose keys (`{"pin_number":0, "domain_owner":"PROGRAMMER", ...}`): Exceeds 2 KB buffer, requiring chunked transfer for simple pin queries.

### 4. Dual-Family Addressable LED Engine (1-Wire PIO & 2-Wire SPI)
- **Problem**: Addressable LEDs span strict-timing single-wire NRZ (WS2812/SK6812) and clocked SPI (APA102/DotStar) with varying color channel layouts (RGB, GRB, RGBW).
- **Decision**:
  - Implement a 4-instruction universal `neopixel.pio` with 10-cycle bit timing (T1=3, T2=4, T3=3) and configurable clock divider (8 MHz for 800 kHz, 4 MHz for 400 kHz).
  - Support 2-wire APA102/SK9822 over hardware SPI with 32-bit start/end frames and 5-bit global brightness scaling.
  - Limit maximum strip buffer to 256 pixels (1,024 bytes) in SRAM with raw pixel streaming (no in-firmware pattern generators).
  - Include an on-the-fly swizzling pipeline supporting RGB, GRB, BGR, RGBW, and GRBW.
- **Alternatives Considered**: Supporting >1024 pixels with background DMA chaining: Deferred to future matrix display extensions to conserve SRAM for `captureBuffer`.

---

## Architecture Diagram

```
                 Incoming HTTP Request (Port 80)
                              │
                    ┌─────────▼─────────┐
                    │  fs_open_custom   │
                    └─────────┬─────────┘
                              │
             Is URI under /api/v1/ or /api/profiles?
             ├── YES ─────────────────────────────┐
             │                                    ▼
             ▼                            [ rest_api_handle ]
     (Static / ROM files)                         │
     e.g. LittleFS / RomFS                        │
                                  ┌───────────────┴───────────────┐
                                  ▼                               ▼
                          [ Tier 1: Static JSON ]        [ Tier 2: Stream Generator ]
                          • /api/v1/profiles             • /api/v1/analyzer/vcd
                          • /api/v1/pins                 • /api/v1/analyzer/data
                          • /api/v1/bus/*                • /api/v1/programmer/read
                          • /api/v1/programmer/target             │
                          • /api/v1/device/*                      ▼
                                  │                       fs_read_custom(file, buf, count)
                                  ▼                       Direct DMA/Buffer -> TCP pbuf
                        Build HTTP 200/201/404            (Zero extra RAM buffering)
                        in rest_response_buffer
```

---

## Risks / Trade-offs

- **[Risk]** Heavy concurrent bus traffic or capture while handling REST HTTP requests.
  - **Mitigation**: RP2040/RP2350 uses Core 0 for networking/USB and Core 1 for time-critical PIO DMA capture. Pin arbitration (`pin_owner.c`) guarantees atomic ownership checks before initiating any bus transaction.
- **[Risk]** lwIP TCP pbuf exhaustion during continuous VCD streaming.
  - **Mitigation**: `fs_read_custom` only produces data when lwIP signals available output window (`count > 0`). If TCP slows down, streaming naturally throttles without packet dropping.
