## Context

AgentLA previously bundled its web desktop assets (`index.html`, `apps.html`, `style.css`, `themes.*`, `app.js`, SVGs) into `firmware-rs` as compile-time string and byte literals via `include_str!` and `include_bytes!`. While convenient during early bring-up, this tightly couples frontend assets to the firmware binary, balloons ROM consumption to $\sim 980\,\text{KB}$, and forces a full Rust build and probe flash for minor CSS/HTML adjustments.

Furthermore, running `picoserve` with a single listening TCP socket caused socket starvation when browsers opened parallel connections to fetch stylesheets and scripts, leading to `Connection refused` errors and unstyled page rendering.

This design introduces a **Dual LittleFS Partition Architecture** that isolates the System WebUI from User Apps, eliminates hardcoded ROM web assets, enables live HTTP web updates, and adds a multi-worker socket pool.

---

## Goals / Non-Goals

**Goals:**
- **Dual Partitioning**: Partition 1 (512 KB, fixed across all hardware) for System WebUI; Partition 2 (remaining dynamic flash capacity) for user apps and data.
- **WebUI Decoupling**: Stream all Web Desktop assets dynamically from Partition 1 (`SYSTEM_FS`) instead of ROM literals.
- **Live HTTP Updates**: Provide `POST /api/v1/system/webui/update` to stream a `.tar` bundle directly into Partition 1 in seconds.
- **Theme Consistency**: Set default `data-theme="forest"` on static HTML markup and synchronize theme selection between Workbench and Launcher.
- **Concurrent TCP Workers**: Run 4 concurrent worker sockets on port 80 to reliably handle parallel browser asset requests.
- **ROM Size Optimization**: Reduce firmware ROM binary size from $\sim 980\,\text{KB}$ to $\sim 350\,\text{KB}$.

**Non-Goals:**
- Modifying LittleFS internal driver code (we use standard `littlefs2` crate instances).
- Implementing an on-probe gzip decompressor (plain ustar `.tar` archives are streamed directly).

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

### 4. Packager Utility for Factory Flashing (`tools/pack_webui.py`)
- **Decision**: Create a Python utility that builds a raw 512 KB LittleFS binary image from `web-ui/` and converts it to UF2 at offset `0x10190000`.
- **Rationale**: Enables building factory-ready combined UF2 binaries (`picotool uf2 merge`) so end users can flash both firmware and WebUI in one drag-and-drop step.

---

## Risks / Trade-offs

- **[Risk] Unformatted System Partition on First Boot**: If Partition 1 is completely empty or unformatted on a brand-new bare chip, `SYSTEM_FS` mount fails.
  - *Mitigation*: The firmware auto-detects unformatted status and formats Partition 1. A factory UF2 combines both `firmware.elf` (at `0x10000000`) and `webui.img` (at `0x10190000`), or prompts for WebUI upload over a lightweight recovery page.
- **[Risk] Flash Sector Wear during WebUI Updates**: Writing to LittleFS causes flash erase cycles.
  - *Mitigation*: WebUI updates are infrequent administrative events, and LittleFS features built-in dynamic wear leveling across all 128 sectors of Partition 1.
- **[Risk] RAM Consumption of 4 Picoserve Workers**: Each worker allocates an HTTP buffer and TCP RX/TX buffers.
  - *Mitigation*: 4 workers with 1.5 KB TCP buffers consume $\sim 20\,\text{KB}$ of RAM. The RP2350 has 520 KB of SRAM with $>400\,\text{KB}$ of headroom.
