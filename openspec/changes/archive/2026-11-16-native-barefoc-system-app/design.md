## Context

AgentLA hosts canonical first-party instruments (`/scope` for logic analysis & oscilloscope, `/terminal` for serial console, `/files` for filesystem storage) in the immutable `SYSTEM_FS` partition at flash offset `0x00204000`. These instruments are pre-compressed with gzip, mounted by `firmware-rs/src/fs_http.rs`, and routed directly by the Picoserve HTTP engine.

BareFOC Studio is currently located in `apps/barefoc/index.html` and installed into the mutable `USER_FS` LittleFS partition (`/apps/barefoc/`). This design promotes BareFOC Studio to a native in-ROM system asset at `webui/barefoc.html`, accessible at `/motor`, `/foc`, and `/barefoc`, and integrates it into the primary top navigation rail and dock.

## Goals / Non-Goals

**Goals:**
- Promote `apps/barefoc/index.html` to `webui/barefoc.html` as a permanent system asset.
- Register routes `/motor`, `/foc`, and `/barefoc` in `firmware-rs/src/web.rs` utilizing `SystemWebFile`.
- Add the **Motor** navigation item to the top navigation header across all canonical system pages.
- Integrate BareFOC Studio into the macOS-style dock and application drawer in `webui/agentla-bridge.js`.
- Bundle `barefoc.html` into the `SYSTEM_FS` build via `tools/pack_webui.py`.
- Maintain strict `#![no_std]` and zero emoji standards.

**Non-Goals:**
- Running motor control loops or embedding FOC commutation logic directly on the AgentLA microcontroller.
- Altering the 512 KB `SYSTEM_FS` or 1,024 KB firmware execution slot boundaries.

## Decisions

### D1: Route Name `/motor` with Aliases
- **Decision**: Establish `/motor` as the canonical route, with `/foc` and `/barefoc` as permanent redirects.
- **Rationale**: Follows the single-word naming convention of existing system instruments (`/home`, `/scope`, `/terminal`, `/files`, `/device`).
- **Alternatives Considered**: `/barefoc` as primary; rejected to maintain clean, technical workbench terminology.

### D2: Direct `SYSTEM_FS` Asset Inclusion
- **Decision**: Place `barefoc.html` directly in `webui/` so it is processed, gzipped, and embedded by `tools/pack_webui.py`.
- **Rationale**: Reuses the high-efficiency `SystemWebFile` streaming handler which automatically serves `.gz` pre-compressed assets when supported by the client browser.
- **Alternatives Considered**: Inlining as a Rust raw byte slice in firmware; rejected because storing HTML in code bloats the 1,024 KB firmware slot when 512 KB `SYSTEM_FS` has ample headroom.

### D3: Unified Top-Rail Navigation
- **Decision**: Update navigation bars across all system HTML pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`) to include the `Motor` tab.
- **Rationale**: Provides consistent single-click context switching across the workbench.

## Risks / Trade-offs

- **[Risk] `SYSTEM_FS` partition exhaustion**
  - **Mitigation**: Total gzipped size of all system assets including `barefoc.html` is ~425 KB, well below the 512 KB partition cap.
- **[Risk] Cache invalidation across firmware/webui updates**
  - **Mitigation**: `SystemWebFile` marks HTML files with `Cache-Control: no-cache, must-revalidate` ensuring clients immediately receive updated markup.
