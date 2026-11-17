## Context

AgentLA hosts canonical first-party instruments (`/scope` for logic analysis & oscilloscope, `/terminal` for serial console, `/files` for filesystem storage, `/motor` for BareFOC Studio) in the immutable `SYSTEM_FS` partition at flash offset `0x00204000`. These instruments are pre-compressed with gzip, mounted by `firmware-rs/src/fs_http.rs`, and served directly by the Picoserve HTTP engine.

Servo Master is currently located in `apps/servo/index.html` and installed into the mutable `USER_FS` LittleFS partition (`/app/servo/`). However, the RP2350 firmware kernel already contains native first-class driver support for RC PWM servos (`pin_owner::Driver::Servo`) and dedicated `/api/v1/servo/*` REST endpoints. This design promotes Servo Master to a native in-ROM system asset at `webui/servo.html`, accessible at `/servo` (with `/servomaster` alias), and integrates it into the primary top navigation rail and dock.

## Goals / Non-Goals

**Goals:**
- Promote `apps/servo/index.html` to `webui/servo.html` as a permanent system asset in ROM.
- Register routes `/servo` and `/servomaster` in `firmware-rs/src/web.rs` utilizing `SystemWebFile`.
- Add the **Servo** navigation item to the top navigation header across all canonical system pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`).
- Integrate Servo Master into the floating dock and URL synchronizer in `webui/agentla-bridge.js` and `webui/app.js`.
- Bundle `servo.html` into the `SYSTEM_FS` build via `tools/pack_webui.py`.
- Ensure tight coupling with the Emergency Safe State (`Shift+Esc`) and target rail telemetry (`/api/v1/system/telemetry`).
- Maintain strict `#![no_std]`, no CSS dot indicators, and zero emoji standards.

**Non-Goals:**
- Removing the low-level `/api/v1/servo/*` REST API endpoints.
- Altering the 512 KB `SYSTEM_FS` or 1,024 KB firmware execution slot boundaries.
- Deprecating or altering BareFOC Studio (`/motor`).

## Decisions

### D1: Route Name `/servo` with `/servomaster` Alias
- **Decision**: Establish `/servo` as the canonical route, with `/servomaster` as a redirect alias.
- **Rationale**: Follows the single-word technical naming convention of existing system instruments (`/home`, `/scope`, `/terminal`, `/files`, `/motor`).
- **Alternatives Considered**: `/servomaster` as primary; rejected to maintain clean, technical workbench terminology.

### D2: Direct `SYSTEM_FS` Asset Inclusion
- **Decision**: Place `servo.html` directly in `webui/` so it is processed, gzipped, and embedded by `tools/pack_webui.py`.
- **Rationale**: Reuses the high-efficiency `SystemWebFile` streaming handler which automatically serves `.gz` pre-compressed assets when supported by client browsers.
- **Alternatives Considered**: Inlining as a Rust byte slice in firmware; rejected because storing HTML in code bloats the firmware slot when `SYSTEM_FS` has ample headroom.

### D3: Unified Top-Rail Navigation
- **Decision**: Update navigation bars across all system HTML pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`) to include the `Servo` tab.
- **Rationale**: Provides consistent single-click context switching across all physical instrumentation views.

### D4: Safe State & Stall Telemetry Integration
- **Decision**: Connect the existing `btn-emergency-safe` action directly to `AgentLA.pins.release()` and detach all active servo PWM channels on trigger.
- **Rationale**: Prevents servo runaway, physical gear damage, and electrical brownouts during bench test failures.

## Risks / Trade-offs

- **[Risk] `SYSTEM_FS` partition exhaustion**
  - **Mitigation**: Total gzipped size of `servo.html` is ~8.2 KB. Total system assets remain under 390 KB, comfortably below the 512 KB partition limit.
- **[Risk] Cache invalidation across firmware/webui updates**
  - **Mitigation**: `SystemWebFile` marks HTML files with `Cache-Control: no-cache, must-revalidate` ensuring clients immediately receive updated markup.
- **[Risk] Top bar navigation overflow on narrow screens**
  - **Mitigation**: The top navbar links are hidden on mobile (`hidden sm:flex`) and accessible via the floating dock and app launcher drawer.
