## Why

Currently, all HTML, CSS, JavaScript, and SVG web assets ($\sim 400\,\text{KB}$) are hardcoded directly into the `firmware-rs` executable binary via `include_str!` and `include_bytes!`. Any UI styling fix, theme tweak, or script update requires a full Rust recompile and a 30-second physical firmware flash. Furthermore, storing the WebUI in executable ROM bloats the binary and limits available code space.

Moving to a **Dual LittleFS Partition Architecture** decouples the WebUI completely from the firmware:
1. **Partition 1 (`SYSTEM_FS`, fixed 512 KB across all hardware)**: Dedicated, read-only partition holding the System WebUI (`index.html`, `apps.html`, `style.css`, `themes.*`, `app.js`). Stores pre-compressed `.gz` assets (~165 KB total) with the **complete DaisyUI component library**, leaving ~347 KB of headroom. It can be updated live over HTTP in 2 seconds without touching the firmware binary.
2. **Partition 2 (`USER_FS`, dynamic remainder of flash)**: Dedicated read/write partition for user-installed Web Apps (`/apps/*`), hardware profiles, and captured signal traces. Users can format or modify user storage without any risk to the System WebUI.
3. **Complete DaisyUI Library for User Apps**: The system bundle provides all DaisyUI components (`steps`, `timeline`, `chat`, `diff`, `stat`, `rating`, `radial-progress`, `range`, `toggle`, `dropdown`, `kbd`, `accordion`, `table`, etc.) and all 32 themes. User apps hosted under `/apps/<name>/` link directly to `/style.css` and `/themes.css`, gaining a rich component catalog with zero CSS bundling overhead in the user partition.

---

## What Changes

1. **Dual LittleFS Partition Geometry in Flash (`storage.rs`)**:
   - Split flash storage into two isolated filesystems:
     - **Partition 1 (`SYSTEM_FS`)**: Offset `0x00190000..0x00210000` (512 KB = 128 sectors of 4 KB). Mounted read-only for daily operation.
     - **Partition 2 (`USER_FS`)**: Offset `0x00210000..FLASH_END - 16KB` (~1.92 MB on Pico 2 / ~13.92 MB on BakuProbe). Mounted read/write for user apps.
2. **WebUI Decoupling & Dynamic Gzip HTTP Streaming (`web.rs`, `fs_http.rs`)**:
   - Remove hardcoded `include_str!` assets from `web_assets.rs`.
   - Bundle the **complete DaisyUI component library** (all components + all 32 themes) in `/style.css` and `/themes.css`.
   - Store assets pre-compressed (`.gz`) inside `SYSTEM_FS`. `picoserve` streams Web Desktop assets directly out of `SYSTEM_FS` with `Content-Encoding: gzip` in 512-byte chunks when requested with `Accept-Encoding: gzip`.
   - User apps continue streaming out of `USER_FS` via `/apps/{app_name}/...` and inherit all DaisyUI styling from `/style.css`.
3. **Live WebUI Update Endpoint (`POST /api/v1/system/webui/update`)**:
   - Accepts a streaming `.tar` archive of web assets and unpacks it directly into `SYSTEM_FS` in place.
   - Enables instant WebUI updates from any browser or curl without touching debuggers or `picotool`.
4. **Theme & Style Synchronization (`web-ui/`)**:
   - Both `index.html` and `apps.html` default to `data-theme="forest"` (with `color-scheme: dark`) to eliminate unstyled initial flashes.
   - Theme configuration is synchronized between the Workbench and the Launcher via shared persistent storage.
5. **Concurrent Multi-Worker HTTP Sockets (`web.rs`)**:
   - Run 4 concurrent worker sockets on port 80 to eliminate TCP socket exhaustion under parallel browser asset fetching.

---

## Capabilities

### New Capabilities
- `dual-littlefs-partitions`: Independent 512 KB Partition 1 (`SYSTEM_FS`) for System WebUI and dynamic Partition 2 (`USER_FS`) for user applications and data.
- `live-webui-updates`: REST endpoint `POST /api/v1/system/webui/update` for updating the System WebUI over HTTP without reflashing firmware.
- `webui-decoupling-from-rom`: Serving web assets directly from `SYSTEM_FS` rather than hardcoded `include_str!` ROM embeds.

### Modified Capabilities
- `static-portal-deployment`: Web assets are streamed dynamically from the dedicated LittleFS `SYSTEM_FS` partition rather than static compilation into firmware ROM.
- `bench-theme-engine`: Themes and styles are synchronized across both the Lab Workbench and App Launcher from persistent flash storage with clean default dark theme (`forest`).

---

## Impact

- **Firmware Binary Size**: Drops from $\sim 980\,\text{KB}$ down to $\sim 350\,\text{KB}$, freeing over $1\,\text{MB}$ of flash ROM for future firmware engines.
- **Partition Footprint & Headroom**: The entire System WebUI with complete DaisyUI components compresses to $\sim 165\,\text{KB}$ via gzip, easily fitting into the fixed 512 KB `SYSTEM_FS` partition with $>340\,\text{KB}$ of headroom.
- **Network Transfer Velocity**: Pre-compressed gzip streaming cuts HTTP transfer payload from $\sim 830\,\text{KB}$ to $\sim 165\,\text{KB}$, making initial page load over USB 4× faster.
- **Zero-Footprint User Apps**: 3rd-party user applications can instantiate any DaisyUI component without bundling duplicate CSS frameworks.
- **Update Velocity**: WebUI updates can be deployed in 2 seconds over HTTP without needing Rust toolchains, `cargo`, or `picotool`.
- **Format Safety**: Users can format the user app partition without affecting the System WebUI.
- **Zero Breaking Changes**: All existing REST routes (`/`, `/apps`, `/api/v1/*`, `/mcp`) remain 100% backward-compatible.
