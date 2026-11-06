## 1. Dual LittleFS Partition Storage Infrastructure (`storage.rs`)

- [x] 1.1 Refactor `storage.rs` to define `SYSTEM_FS` (512 KB, offset `0x00190000`) and `USER_FS` (offset `0x00210000` to flash end).
- [x] 1.2 Implement separate mutexes or access helpers (`with_system_fs` and `with_user_fs`) with independent LittleFS driver instances.
- [x] 1.3 Add format protection logic ensuring user partition operations can never touch or reformat `SYSTEM_FS`.

## 2. Full DaisyUI Bundle & System WebUI Packaging Tool (`tools/pack_webui.py`)

- [x] 2.1 Bundle the complete DaisyUI component catalog and themes into `web-ui/style.css` and `web-ui/themes.css`.
- [x] 2.2 Create `tools/pack_webui.py` to pre-compress all assets with gzip (`*.gz`) and format them into a 512 KB LittleFS image (`webui.img`), generating `webui.uf2` at offset `0x10190000`.
- [x] 2.3 Verify that total gzipped image size is $\sim 165\,\text{KB}$, leaving $>340\,\text{KB}$ of headroom in the 512 KB partition.

## 3. Dynamic HTTP Gzip Streaming & Live Update Endpoint (`web.rs`, `fs_http.rs`)

- [x] 3.1 Update `fs_http.rs` and `web.rs` routes (`/`, `/apps`, `/style.css`, `/themes.css`, `/app.js`, SVGs) to stream `.gz` assets from `SYSTEM_FS` with `Content-Encoding: gzip` when requested.
- [x] 3.2 Implement `POST /api/v1/system/webui/update` endpoint using the streaming `TarUnpacker` to update `SYSTEM_FS` live over HTTP.
- [x] 3.3 Remove static `include_str!` and `include_bytes!` WebUI embeds from `web_assets.rs`, verifying binary size drops to ~350 KB.

## 4. Theme Markup Sync & Multi-Worker Verification

- [x] 4.1 Update `web-ui/index.html` and `web-ui/apps.html` to default cleanly to `data-theme="forest"` with `color-scheme: dark`.
- [x] 4.2 Verify 4-worker concurrent socket pool in `web.rs` handles parallel browser asset requests without connection drops.
- [x] 4.3 Recompile release UF2, flash physical RP2350 probe, and verify live via browser and HTTP tests.
