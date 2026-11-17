## Context

AgentLA hosts canonical first-party instruments in the immutable `SYSTEM_FS` partition at flash offset `0x00204000`. These instruments (`/scope`, `/terminal`, `/files`, `/motor`, `/servo`) are pre-compressed with gzip, mounted by `firmware-rs/src/fs_http.rs`, and served directly by the Picoserve HTTP engine.

NeoPixel Control is currently located in `apps/neopixel/index.html` and installed into the mutable `USER_FS` LittleFS partition (`/app/neopixel/`). The RP2350 firmware kernel already contains native driver support for WS2812 and clocked LEDs (`crate::bridge::ws2812`, `crate::instruments::ws2812`, and WebSocket endpoint `/api/v1/led/ws`). However, in the MCP server, `ws2812_transmit` is currently a parameter-less stub that only sends 3 hardcoded colors.

This design promotes NeoPixel Control to a native in-ROM system asset at `webui/neopixel.html`, accessible at `/neopixel` (with `/led` alias), integrates the Solar **Bold Duotone Filters** icon into the dock, and equips the MCP server with 5 functional tools for addressable LED actuation.

## Goals / Non-Goals

**Goals:**
- Promote `apps/neopixel/index.html` to `webui/neopixel.html` as a permanent system asset in ROM.
- Register routes `/neopixel` and `/led` in `firmware-rs/src/web.rs` utilizing `SystemWebFile`.
- Add the **NeoPixel** navigation item to the top navigation header across all canonical system pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`, `neopixel.html`).
- Integrate NeoPixel Control into the floating dock and application drawer in `webui/agentla-bridge.js` and `webui/app.js` using the Solar icon **Bold Duotone Filters**.
- Bundle `neopixel.html` into the `SYSTEM_FS` build via `tools/pack_webui.py`.
- Declare and dispatch 5 MCP tools:
  - `neopixel_fill`: Fill N pixels with a uniform color.
  - `neopixel_set`: Update a single pixel at a specified index.
  - `neopixel_clear`: Blank / power-off all pixels.
  - `neopixel_pattern`: Drive procedural patterns (`rainbow`, `cylon`, `fire`, `meteor`, `pulse`, `plasma`).
  - `neopixel_analyze`: Measure WS2812 signal timing margins.
  - `neopixel_transmit`: Transmit an arbitrary array of colors.
- Maintain strict `#![no_std]`, no CSS dot indicators, and zero emoji standards.

**Non-Goals:**
- Removing the low-level `/api/v1/led/ws` WebSocket or REST API endpoints.
- Altering the 512 KB `SYSTEM_FS` or 1,024 KB firmware execution slot boundaries.

## Decisions

### D1: Route Name `/neopixel` with `/led` Alias
- **Decision**: Establish `/neopixel` as the canonical route, with `/led` as a 301 redirect alias.
- **Rationale**: Aligns with existing app naming and user familiarity, while providing `/led` for fast manual navigation.

### D2: Iconography Selection
- **Decision**: Use the Solar icon **Bold Duotone Filters** (`ref/solar-icons/packages/core/svgs/tools/BoldDuotone/filters.svg`) for the dock, launcher tile, and header branding.
- **Rationale**: The overlapping optical filter lenses directly symbolize RGB additive color blending. Implemented using standard `currentColor` with `opacity="0.5"` duotone fills without floating CSS dot indicators.

### D3: Unified Top-Rail Navigation
- **Decision**: Update navigation bars across all system HTML pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`, `neopixel.html`) to include the `NeoPixel` tab.
- **Rationale**: Provides consistent single-click context switching across all physical instrumentation views.

### D4: MCP Tool Design
- **Decision**: Provide structured, intuitive tools (`neopixel_fill`, `neopixel_set`, `neopixel_clear`, `neopixel_pattern`, `neopixel_analyze`, `neopixel_transmit`) supporting hex color strings (e.g. `"#FF0000"`) and RGB triplet arrays (`[255, 0, 0]`).
- **Rationale**: AI agents can immediately execute intuitive actions like "set the status LED to green" or "run a rainbow pattern at 50% speed" without having to assemble binary byte streams.

## Risks / Trade-offs

- **[Risk] `SYSTEM_FS` partition exhaustion**
  - **Mitigation**: Total gzipped size of `neopixel.html` is ~13.8 KB. Total system assets remain under 410 KB, comfortably below the 512 KB partition limit with >115 KB remaining.
- **[Risk] Cache invalidation across firmware/webui updates**
  - **Mitigation**: `SystemWebFile` marks HTML files with `Cache-Control: no-cache, must-revalidate` ensuring clients immediately receive updated markup.
