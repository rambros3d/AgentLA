## Why

AgentLA currently hosts a single, monolithic WebUI compiled directly into firmware ROM fsdata. Users requiring specialized frontends (e.g. automotive CAN sniffers, automated battery profilers, production test fixtures, or custom education dashboards) currently have no clean way to install or host custom web interfaces on the device without recompiling and reflashing the entire firmware.

Furthermore, while the `/mcp` endpoint serves AI coding agents via JSON-RPC 2.0, standard web applications and external integrations require a predictable, first-class REST API (`/api/v1/...`).

Introducing LittleFS flash storage, namespaced URL routing (`/apps/{app-name}`), a built-in ROM App Launcher (`/apps`), a dedicated REST API, and a browser-orchestrated Community App Store unlocks a vibrant ecosystem where users can discover, install, launch, and manage custom hardware tools directly from their browser with zero configuration on the microcontroller.

## What Changes

- **Preserve Factory Root WebUI**: The root URL `http://10.10.10.10/` permanently serves the factory WebUI compiled into ROM fsdata (`pico_fsdata.inc`), guaranteeing that the device is unbrickable and the default hardware console is always accessible.
- **Dedicated REST API (`/api/v1/*`)**: Implement clean, idiomatic REST endpoints returning JSON and standard HTTP status codes for system status, pin controls, instrument operations, hardware profiles, and app lifecycle management alongside the existing `/mcp` agent endpoint.
- **Flash Storage Partitioning (LittleFS)**: Allocate a dedicated, wear-leveled LittleFS filesystem partition in flash (~2.38 MB on Pico 2 4MB flash; ~14.38 MB on BakuProbe 16MB flash) for user web applications under `/apps/`.
- **URL Routing Engine**: Extend lwIP with a custom file handler (`LWIP_HTTPD_CUSTOM_FILES`) to route:
  - `GET /` and root assets $\rightarrow$ Factory WebUI in ROM fsdata.
  - `GET /apps` $\rightarrow$ Built-in App Launcher in ROM fsdata.
  - `GET /apps/{app-name}/*` $\rightarrow$ LittleFS files with SPA fallback to `index.html`.
  - `GET|POST|DELETE /api/v1/*` $\rightarrow$ REST API handlers.
  - `POST /mcp` $\rightarrow$ JSON-RPC 2.0 MCP agent server.
- **Dual-Format App Packages**: Support streaming `.tar` archives (with `app.json`, `index.html`, and static assets) and single standalone `.html` files. The on-device streaming tar unpacker requires zero RAM buffering and streams directly into LittleFS.
- **Browser-Mediated App Store**: A catalog viewer within the App Launcher that fetches a public community `apps.json` catalog from GitHub/CDN, renders searchable app cards, and coordinates 1-click installation directly from the browser without requiring TLS or internet access on the RP2350 microcontroller.
- **App Installer Confirmation Modal**: Client-side TAR inspection displaying app icon, description, author, version, homepage, hardware compatibility, and mandatory flash storage availability verification prior to installation.

## Capabilities

### New Capabilities
- `multi-app-littlefs-storage`: Flash layout partitioning for LittleFS, persistent storage for user webapps under `/apps/{app-name}/*`, and storage usage query API.
- `url-routing-engine`: lwIP custom file hook routing `/` to factory ROM fsdata, `/apps` to built-in ROM App Launcher, `/apps/{app-name}/*` to LittleFS files with SPA routing fallback, and `/api/v1/*` to dedicated REST API.
- `dedicated-rest-api`: First-class RESTful HTTP API (`/api/v1/...`) for hardware control, pin states, profile management, and app lifecycle without JSON-RPC wrappers.
- `community-app-store`: Browser-orchestrated App Store at `/apps` fetching remote catalog JSON (`apps.json`), displaying app cards, updates, and custom registry URLs.
- `app-installer-modal`: Interactive pre-install modal inspecting TAR/manifest contents client-side, verifying flash capacity, checking hardware compatibility, and streaming to device.

### Modified Capabilities
- `network-access`: Extend network routing to handle `/api/v1` REST paths, `/apps` custom file resolution, and multipart/streaming tar uploads.

## Impact

- **Firmware Size & Memory**: Adds LittleFS driver and lightweight streaming TAR unpacker (~10 KB flash, 512 bytes streaming RAM).
- **Network Stack**: `lwipopts.h` enables `LWIP_HTTPD_CUSTOM_FILES 1`.
- **Flash Layout**: Reserves flash range from 1.6MB to end-16KB for LittleFS partition; leaves hardware profile slots (1.5MB-1.56MB) and factory boot/crash blocks untouched.
- **Web Frontend**: Adds App Launcher (`firmware/html/apps.html`) to factory ROM fsdata and top navigation link to `/apps`.
