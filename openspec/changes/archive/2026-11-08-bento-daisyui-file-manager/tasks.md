## 1. Firmware REST API Endpoints

- [x] 1.1 Implement `GET /api/v1/fs/stat` returning byte metrics and read-only status for `system_fs` and `user_fs`
- [x] 1.2 Implement `GET /api/v1/fs/list` directory traversal returning JSON entry metadata (`name`, `size`, `is_dir`)
- [x] 1.3 Implement `GET /api/v1/fs/read` chunked 512-byte streaming file reader with Content-Type detection
- [x] 1.4 Implement mutating endpoints (`/api/v1/fs/write`, `DELETE /api/v1/fs/entry`, `/api/v1/fs/mkdir`, `/api/v1/fs/rename`) with strict 403 Forbidden rejection when `root=system`

## 2. Standalone Frontend (webui/files.html)

- [x] 2.1 Scaffold `webui/files.html` with DaisyUI 5, instant theme restoration script, and top navigation header
- [x] 2.2 Build Bento Storage Stats section with `stats` and `progress` bars displaying live partition utilization
- [x] 2.3 Implement interactive breadcrumbs (`.breadcrumbs`) and root partition selector (`.join`)
- [x] 2.4 Implement responsive file canvas supporting squircle grid view (`.card`) and details list view (`.table`)
- [x] 2.5 Add frontend permission guards (disabling write actions, displaying `Read-Only` badges for `system_fs`)
- [x] 2.6 Implement in-place file viewer and editor modal (`.modal`) with monospace textarea and syntax support
- [x] 2.7 Implement drag-and-drop file uploader and new folder creation dialog for `user_fs`

## 3. WebUI Integration & Packaging

- [x] 3.1 Register HTTP routes `/files` and `/files.html` in `firmware-rs/src/web.rs` and `fs_http.rs`
- [x] 3.2 Add "Files" squircle application tile to `webui/index.html` under System Instruments and allow dock pinning
- [x] 3.3 Update `tools/pack_webui.py` to pre-compress `files.html` into `files.html.gz` within `SYSTEM_FS`

## 4. Verification & Validation

- [x] 4.1 Validate frontend functionality in static demo mode with mock filesystem responses
- [x] 4.2 Verify `#![no_std]` compilation (`cargo check --target thumbv8m.main-none-eabihf`)
- [x] 4.3 Verify security boundary: confirm mutating operations targeting `system_fs` return HTTP 403 Forbidden
