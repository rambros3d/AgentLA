## Context

AgentLA runs on the Raspberry Pi RP2350 microcontroller (ARM Cortex-M33 / RISC-V Hazard3), presenting a virtual USB RNDIS/CDC-ECM ethernet interface mapped to `10.10.10.10`. The firmware utilizes lwIP for networking and currently serves a factory WebUI compiled into ROM fsdata (`pico_fsdata.inc`).

Users need specialized frontends (CAN bus protocol analyzers, battery discharge profilers, sensor visualization boards, automated test jigs) that can be installed on the device and accessed without overwriting or bricking the core firmware. Furthermore, an intuitive App Store allows users to browse and install community-created applications from a public catalog, while a dedicated REST API allows third-party tools to interact with the device without being forced through JSON-RPC wrappers.

## Goals / Non-Goals

**Goals:**
- Guarantee the root URL (`http://10.10.10.10/`) permanently serves the default factory WebUI from ROM fsdata.
- Mount a LittleFS flash partition in unused flash (2.38 MB on Pico 2; 14.38 MB on BakuProbe) for multi-tenant user apps under `/apps/{app-name}/`.
- Route `/apps` to a built-in ROM App Launcher & Installer dashboard.
- Provide SPA fallback for custom apps: missing subpaths under `/apps/{app-name}/*` fallback to `/apps/{app-name}/index.html`.
- Provide a dedicated REST API (`/api/v1/...`) for hardware control, pins, profiles, and app lifecycle alongside `/mcp`.
- Implement a browser-mediated Community App Store in the launcher that fetches remote `apps.json`, displays app cards, and coordinates 1-click downloads and installations.
- Provide a pre-install TAR inspection modal that validates app metadata, hardware requirements, and verifies available LittleFS flash storage prior to installation.
- Implement an on-device streaming TAR unpacker with zero RAM buffering (512-byte header stream) directly writing to LittleFS.

**Non-Goals:**
- Implementing direct HTTPS/TLS network clients in the RP2350 firmware (the host browser mediates downloads).
- Backwards compatibility with deprecated legacy API shapes.
- Hosting full server-side runtime scripts (Node.js/Python) on the microcontroller (apps are static SPAs interacting via `/api/v1`).

## Decisions

### 1. Flash Partitioning & LittleFS Integration
- **Decision**: Partition flash starting after profile slots (`0x0019_0000` / 1.6MB offset) up to the crash/boot sectors (`End - 16KB`).
  - *Pico 2 (4MB Flash)*: ~2.38 MB partition.
  - *BakuProbe (16MB Flash)*: ~14.38 MB partition.
- **Rationale**: LittleFS provides wear-leveling, bad-block tolerance, and dynamic file allocation suited for NOR flash. It protects flash longevity during app installations and updates.
- **Alternatives Considered**: Contiguous raw sector allocation (causes fragmentation and lacks filesystem abstractions).

### 2. URL Routing Architecture & SPA Fallback
- **Decision**: Enable `LWIP_HTTPD_CUSTOM_FILES 1` in `lwipopts.h`. In `fs_open_custom(file, path)`:
  - If `path == "/"`, `/style.css`, `/app.js`: return `0` (lwIP falls back to ROM fsdata).
  - If `path == "/apps"` or `/apps/`: return `0` (serves ROM App Launcher `apps.html`).
  - If `path` starts with `/apps/{app-name}/`:
    - Check LittleFS for the exact path `/apps/{app-name}/{asset}`.
    - If found, open LFS file and stream.
    - If not found and the path has no file extension (SPA sub-route like `/apps/can-sniffer/filter/std`), open `/apps/{app-name}/index.html`.
    - If not found and has an extension, return 404.
- **Rationale**: Ensures standard client-side routing (React Router, Vue Router) works without 404 errors while keeping the factory WebUI unbrickable in ROM.

### 3. Dual-Format Package Installation & Streaming TAR Unpacker
- **Decision**: Support both `.tar` multi-file packages and `.html` single-file dashboards at `POST /api/v1/apps/install?name={name}`.
  - For `.tar`: Parse 512-byte headers iteratively from lwIP POST pbuf chunks. Each file header provides filename and byte count; file data is immediately written to LittleFS. Zero RAM buffer required beyond one 512-byte header struct.
  - For `.html`: Write payload directly to `/apps/{name}/index.html`.
- **Rationale**: Avoids memory exhaustion on RP2350. ZIP archives require DEFLATE decompression buffers (~32KB RAM minimum), whereas uncompressed TAR requires zero decompression memory.

### 4. Browser-Mediated Community App Store & Catalog
- **Decision**: The browser executing `/apps` fetches public `apps.json` directly from GitHub or CDN, fetches the selected app archive, and streams it to the microcontroller via HTTP POST.
- **Rationale**: The RP2350 does not require TLS certificates, external DNS, or internet NAT/gateway configuration on the host. It works seamlessly out-of-the-box on Linux, Windows, and macOS.
- **Custom Catalogs**: Users can supply custom catalog JSON URLs or paste direct `.tar`/`.html` links stored in `localStorage`.

### 5. Pre-Install Modal with Mandatory Flash Capacity Check
- **Decision**: When a TAR file is selected or an App Store install is initiated, a client-side tar scanner extracts `app.json` and `icon.svg` in browser memory before uploading.
- **Modal Validation Criteria**:
  1. **Flash Space Check**: Fetches `/api/v1/apps/storage`. If `required_bytes > free_bytes`, the Install button is disabled and an explicit "Insufficient Flash Space" alert is displayed.
  2. **Hardware Requirement Check**: Checks required peripherals against active board manifest (e.g. CAN transceiver). If missing, displays an advisory with a confirmation override checkbox.
  3. **Version & Overwrite Notice**: Warns if an existing app will be overwritten.

### 6. Dedicated REST API (`/api/v1/*`)
- **Decision**: Implement RESTful endpoints alongside `/mcp`:
  - `GET /api/v1/apps`: List installed apps.
  - `GET /api/v1/apps/storage`: Query total, used, and free flash bytes.
  - `POST /api/v1/apps/install?name={app}`: Stream-install an app.
  - `DELETE /api/v1/apps/{app}`: Uninstall app and reclaim flash.
  - `GET /api/v1/system`: MCU info, flash size, board name.
  - `GET /api/v1/pins`: Read pin configurations and states.
  - `POST /api/v1/pins/safe`: Trigger emergency Safe State (quiesce all pins).

## Risks / Trade-offs

- **[Risk] LittleFS block writes stall lwIP thread during large tar install** → *Mitigation*: Process TAR chunks inside network task cooperatively with watchdog reset and feed progress back in response stream.
- **[Risk] Browser runs out of memory on huge tar archives** → *Mitigation*: TAR header preview reads only the first few 512-byte blocks to find `app.json`; app archives are typically < 500 KB.
- **[Risk] App corrupts flash filesystem on power cut during install** → *Mitigation*: LittleFS is inherently copy-on-write and atomic per transaction; if an install is interrupted, partial directory is marked invalid or replaced on retry.
