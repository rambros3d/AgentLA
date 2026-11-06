## Context

AgentLA previously bundled its web desktop assets (`index.html`, `apps.html`, `style.css`, `themes.*`, `app.js`, SVGs) into `firmware-rs` as compile-time string and byte literals via `include_str!` and `include_bytes!`. While convenient during early bring-up, this tightly couples frontend assets to the firmware binary, balloons ROM consumption to $\sim 980\,\text{KB}$, and forces a full Rust build and probe flash for minor CSS/HTML adjustments.

Furthermore, running `picoserve` with a single listening TCP socket caused socket starvation when browsers opened parallel connections to fetch stylesheets and scripts, leading to `Connection refused` errors and unstyled page rendering.

This design introduces a **Dual LittleFS Partition Architecture** that isolates the System WebUI from User Apps, eliminates hardcoded ROM web assets, enables live HTTP web updates, and adds a multi-worker socket pool.

---

## Goals / Non-Goals

**Goals:**
- **Dual Partitioning**: Partition 1 (512 KB, fixed across all hardware) for System WebUI; Partition 2 (remaining dynamic flash capacity) for user apps and data.
- **Complete DaisyUI Library**: Bundle the entire DaisyUI component catalog (`btn`, `card`, `tabs`, `modal`, `drawer`, `steps`, `timeline`, `chat`, `diff`, `stat`, `rating`, `radial-progress`, `range`, `toggle`, `dropdown`, `kbd`, `accordion`, `table`, etc.) and all 32 themes so user apps inherit comprehensive UI primitives.
- **Pre-compressed Gzip LittleFS Streaming (Approach A)**: Package pre-compressed `.gz` assets (`~165 KB` total) into `SYSTEM_FS` and stream them with `Content-Encoding: gzip`, saving 70% flash space and accelerating USB page loads 4×.
- **WebUI Decoupling**: Stream all Web Desktop assets dynamically from Partition 1 (`SYSTEM_FS`) instead of ROM literals.
- **Live HTTP Updates**: Provide `POST /api/v1/system/webui/update` to stream a `.tar` bundle directly into Partition 1 in seconds.
- **Theme Consistency**: Set default `data-theme="forest"` on static HTML markup and synchronize theme selection between Workbench and Launcher.
- **Concurrent TCP Workers**: Run 4 concurrent worker sockets on port 80 to reliably handle parallel browser asset requests.
- **ROM Size Optimization**: Reduce firmware ROM binary size from $\sim 980\,\text{KB}$ to $\sim 350\,\text{KB}$.

**Non-Goals:**
- Modifying LittleFS internal driver code (we use standard `littlefs2` crate instances).
- Implementing on-the-fly MCU runtime gzip compression (compression is pre-computed at pack time; MCU simply streams the `.gz` bytes).
- Restricting user apps to a limited bespoke CSS subset.

---

## Decisions

### 1. Dual Independent LittleFS Filesystems vs. Single Partition
- **Decision**: Instantiate two separate `littlefs2::fs::Filesystem` instances with distinct flash sector ranges:
  - `SYSTEM_FS`: Offset `0x00190000..0x00210000` (512 KB, 128 sectors of 4 KB).
  - `USER_FS`: Offset `0x00210000..FLASH_END - 16KB` (~1.92 MB on Pico 2 / ~13.92 MB on BakuProbe).
- **Rationale**: Physical sector isolation ensures that user app operations (installations, file writes, or "Format User Storage") can *never* overwrite, corrupt, or erase the System WebUI.
- **Alternatives Considered**: Single partition with software directory guards (`/system` vs `/apps`). Rejected because a filesystem format operation or corruption on a single partition wipes all directories.

### 2. Multi-Worker TCP Socket Architecture in Picoserve
- **Decision**: In `web.rs`, run 4 concurrent workers bound to port 80 using `embassy_futures::join::join4`.
- **Rationale**: Picoserve's `listen_and_serve` serves at most 1 connection at a time per worker. Modern browsers fetch CSS, JS, fonts, and SVGs concurrently across 4–6 parallel sockets. Running 4 workers eliminates `ECONNREFUSED` during page loads and prevents long-lived SSE streams (`/api/v1/oscilloscope/stream`) from starving static file requests.

### 3. Streaming Tarball Installer for System WebUI (`POST /api/v1/system/webui/update`)
- **Decision**: Re-use the existing streaming `ustar` state machine (`TarUnpacker` in `apps.rs`) targeted at `SYSTEM_FS`.
- **Rationale**: Unpacks 512-byte blocks directly from the TCP stream into flash with zero intermediate RAM buffering, allowing full WebUI updates without exceeding bare-metal microcontroller memory limits.

### 4. Packager Utility & Pre-compressed Gzip Pipeline (`tools/pack_webui.py`)
- **Decision**: The packaging utility builds a 512 KB LittleFS binary image containing pre-compressed `.gz` assets (`index.html.gz`, `apps.html.gz`, `style.css.gz`, `themes.css.gz`, `app.js.gz`).
- **Rationale**: Full DaisyUI with all components is ~340 KB uncompressed. Storing it raw alongside workbench scripts (~830 KB total) would exceed a 512 KB partition. Gzipping reduces the total filesystem payload to **~165 KB**, comfortably fitting inside 512 KB with **~347 KB of headroom**. The probe serves these directly with `Content-Encoding: gzip` when the browser sends `Accept-Encoding: gzip`.

### 5. Shared Styling Bus for User Applications
- **Decision**: The system serves full DaisyUI CSS and theme tokens at the root paths `/style.css` and `/themes.css` with long-term browser cache headers (`Cache-Control: public, max-age=31536000`).
- **Rationale**: User applications under `/apps/<name>/` include `<link rel="stylesheet" href="/style.css">`. The browser serves it directly from memory/disk cache without extra HTTP requests. User apps gain immediate access to all DaisyUI components (`steps`, `timeline`, `chat`, `diff`, `stat`, `range`, `kbd`, etc.) while keeping user apps ultra-lightweight (often < 10 KB).

---

## Risks / Trade-offs

- **[Risk] Unformatted System Partition on First Boot**: If Partition 1 is completely empty or unformatted on a brand-new bare chip, `SYSTEM_FS` mount fails.
  - *Mitigation*: The firmware auto-detects unformatted status and formats Partition 1. A factory UF2 combines both `firmware.elf` (at `0x10000000`) and `webui.img` (at `0x10190000`), or prompts for WebUI upload over a lightweight recovery page.
- **[Risk] Flash Sector Wear during WebUI Updates**: Writing to LittleFS causes flash erase cycles.
  - *Mitigation*: WebUI updates are infrequent administrative events, and LittleFS features built-in dynamic wear leveling across all 128 sectors of Partition 1.
- **[Risk] RAM Consumption of 4 Picoserve Workers**: Each worker allocates an HTTP buffer and TCP RX/TX buffers.
  - *Mitigation*: 4 workers with 1.5 KB TCP buffers consume $\sim 20\,\text{KB}$ of RAM. The RP2350 has 520 KB of SRAM with $>400\,\text{KB}$ of headroom.
