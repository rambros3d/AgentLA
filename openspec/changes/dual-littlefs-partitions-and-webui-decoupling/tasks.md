## 1. Dual LittleFS Partition Storage Infrastructure (`storage.rs`)

- [ ] 1.1 Refactor `storage.rs` to define `SYSTEM_FS` (512 KB, offset `0x00190000`) and `USER_FS` (offset `0x00210000` to flash end).
- [ ] 1.2 Implement separate mutexes or access helpers (`with_system_fs` and `with_user_fs`) with independent LittleFS driver instances.
- [ ] 1.3 Add format protection logic ensuring user partition operations can never touch or reformat `SYSTEM_FS`.

## 2. System WebUI Packaging Tool & Factory Image (`tools/`)

- [ ] 2.1 Create `tools/pack_webui.py` to package `web-ui/` files into a 512 KB LittleFS image and generate `webui.uf2` at offset `0x10190000`.
- [ ] 2.2 Verify that `tools/pack_webui.py` correctly includes `index.html`, `apps.html`, `style.css`, `themes.css`, `themes.json`, `app.js`, and SVG icons.

## 3. Dynamic HTTP Streaming & Live Update Endpoint (`web.rs`, `fs_http.rs`)

- [ ] 3.1 Update `web.rs` routes (`/`, `/apps`, `/style.css`, `/themes.css`, `/app.js`, SVGs) to stream chunk-by-chunk from `SYSTEM_FS`.
- [ ] 3.2 Implement `POST /api/v1/system/webui/update` endpoint using the streaming `TarUnpacker` to update `SYSTEM_FS` live over HTTP.
- [ ] 3.3 Remove static `include_str!` and `include_bytes!` WebUI embeds from `web_assets.rs`, verifying binary size drops to ~350 KB.

## 4. Theme Markup Sync & Multi-Worker Verification

- [ ] 4.1 Update `web-ui/index.html` and `web-ui/apps.html` to default cleanly to `data-theme="forest"` with `color-scheme: dark`.
- [ ] 4.2 Verify 4-worker concurrent socket pool in `web.rs` handles parallel browser asset requests without connection drops.
- [ ] 4.3 Recompile release UF2, flash physical RP2350 probe, and verify live via browser and HTTP tests.
