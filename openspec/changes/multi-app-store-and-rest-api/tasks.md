## 1. LittleFS Partition Driver & Flash Storage

- [x] 1.1 Integrate LittleFS C driver and configure block device operations over RP2350 flash
- [x] 1.2 Implement partition geometry calculation (offset 0x0019_0000 to End - 16KB) supporting 4MB (Pico 2) and 16MB (BakuProbe)
- [x] 1.3 Implement LittleFS mount, auto-format on first boot, and `/apps` directory initialization
- [x] 1.4 Implement storage utilization query (`GET /api/v1/apps/storage`) returning total, used, and free bytes

## 2. URL Routing Engine & Custom File Handler

- [x] 2.1 Enable `LWIP_HTTPD_CUSTOM_FILES 1` in `firmware/src/lwipopts.h`
- [x] 2.2 Implement `fs_open_custom` to route root paths to ROM fsdata and `/apps/{app-name}/*` to LittleFS
- [x] 2.3 Implement directory trailing slash redirect (`/apps/{app-name}` -> `/apps/{app-name}/`)
- [x] 2.4 Implement Single Page Application (SPA) routing fallback to `index.html` for non-file subpaths
- [x] 2.5 Implement chunked streaming file reader (`fs_read_custom`) from LittleFS into lwIP pbufs

## 3. Streaming Package Installer & Unpacker

- [x] 3.1 Implement streaming 512-byte TAR header parser writing files directly to `/apps/{app-name}/`
- [x] 3.2 Implement standalone HTML installer writing single files directly to `/apps/{app-name}/index.html`
- [x] 3.3 Implement `POST /api/v1/apps/install?name={app-name}` endpoint handling streaming uploads
- [x] 3.4 Implement `DELETE /api/v1/apps/{app-name}` recursive deletion and flash block reclaim

## 4. Dedicated REST API

- [x] 4.1 Implement `GET /api/v1/apps` returning JSON list of installed apps, versions, and sizes
- [x] 4.2 Implement `GET /api/v1/apps/{app-name}/manifest` returning parsed `app.json`
- [x] 4.3 Implement `GET /api/v1/system` returning board info, flash size, and uptime
- [x] 4.4 Implement `POST /api/v1/pins/safe` quiescing all pins to high-impedance safe state

## 5. Built-in ROM App Launcher UI (`/apps`)

- [x] 5.1 Create `firmware/html/apps.html` compiled into ROM fsdata with modern responsive DaisyUI styling
- [x] 5.2 Implement "My Apps" tab displaying installed apps cards, storage usage meter, launch links, and delete buttons
- [x] 5.3 Add header navigation toggle between Factory Console (`/`) and App Launcher (`/apps`)

## 6. Community App Store & Pre-Install Modal

- [x] 6.1 Implement "App Store" tab fetching remote `apps.json` catalog and rendering searchable community app cards
- [x] 6.2 Implement client-side in-memory TAR reader to extract `app.json` and `icon.svg` prior to upload
- [x] 6.3 Implement Installation Confirmation Modal displaying app icon, description, author, version, homepage, and compatibility
- [x] 6.4 Implement mandatory flash storage capacity check in modal comparing required bytes against `/api/v1/apps/storage`
- [x] 6.5 Implement hardware requirement check against active board manifest with advisory warning and override checkbox
- [x] 6.6 Implement 1-click browser download and streaming push to device with progress feedback

## 7. Verification & Automated Tests

- [x] 7.1 Write Python test script `test/test_multi_app_store.py` testing REST API, TAR installation, storage reporting, and SPA fallback
- [x] 7.2 Verify clean compilation on RP2350 target and test suite execution
