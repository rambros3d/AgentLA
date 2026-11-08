## Context

In AgentLA's dual LittleFS architecture, `SYSTEM_FS` (512 KB at `0x00260000`) stores the System WebUI distribution (`index.html`, `lab.html`, `terminal.html`, `style.css`, `daisyui.css`, `app.js`, etc.). If `SYSTEM_FS` is blank (e.g., brand-new chip, formatted partition, or invalid offset), requests to `/home` or root `/` trigger the fallback path in `SystemWebFile` (`firmware-rs/src/fs_http.rs`). Currently, this path returns an unstyled, plain-text string with instructions but zero interactive recovery actions.

## Goals / Non-Goals

**Goals:**
- Provide a self-contained, interactive Neo-Industrial WebUI recovery interface directly in Flash ROM.
- Enable direct in-browser upload of `webui.tar` to `POST /api/v1/system/webui/update` with progress reporting.
- Enable 1-click GitHub release fetching of `webui.tar` via client-side JavaScript.
- Provide a clear, high-contrast navigation button `[ Open Device Diagnostics (/device) ]` to drop into developer tools.
- Maintain strict `#![no_std]` zero-heap compliance in firmware.

**Non-Goals:**
- Embedding the entire WebUI suite in ROM (the WebUI remains in LittleFS; only this emergency recovery UI is in ROM).
- Modifying the streaming upload handler `SystemWebUiUpdate` (which already functions reliably).

## Decisions

### 1. In-ROM HTML Asset File vs Inline String
- **Decision**: Store the fallback page as `firmware-rs/assets/recovery.html` and embed it in `fs_http.rs` using `include_str!("../assets/recovery.html")` (mirroring `device.html`).
- **Rationale**: Keeps HTML/CSS/JS clean, syntax-highlighted, and testable via pytest without escaping issues inside Rust source code.
- **Alternatives Considered**: Keeping a single huge string literal in `fs_http.rs` (hard to maintain and test).

### 2. Client-Side GitHub API vs Firmware Network Pull
- **Decision**: Perform GitHub release queries and artifact downloading in browser client-side JavaScript, then stream the Blob to the probe.
- **Rationale**: The RP2350 probe is on a private USB CDC-NCM or RNDIS network (`10.10.10.10`) with no direct WAN/Internet routing or TLS certificate stores. The user's workstation browser has Internet connectivity.

### 3. Diagnostics Navigation
- **Decision**: Prominent navigation button linking to `/device` (or `/device.html`).
- **Rationale**: Satisfies the user's explicit requirement: "IN THE FALLBACK PAGE, ALSO ADD A DIAGNOSTICS BUTTON TO SWITCH TO /DEVICE PAGE."

## Risks / Trade-offs

- **[Risk]** Browser has no Internet connection when attempting 1-click GitHub pull.
  - *Mitigation*: The local file picker (`webui.tar`) and `/device` navigation button remain 100% offline-functional. The GitHub pull button displays a clear connection error message if GitHub is unreachable.
- **[Risk]** Accidental firmware bloat.
  - *Mitigation*: The entire `recovery.html` is kept under 2.5 KB uncompressed. Tested and verified in flash map.
