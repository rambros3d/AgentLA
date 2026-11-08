## Why

While AgentLA features a dual LittleFS partition architecture (`SYSTEM_FS` at 512 KB and dynamic `USER_FS` up to ~13.92 MB), users and developers currently have no interactive, visual way to explore files stored on the probe. Inspecting system assets (flash algorithms `.flm`, board manifests, ROM SVGs) or managing user storage (installed micro-apps, custom hardware profiles, captured `.vcd` logic analyzer traces, configuration files) requires indirect API calls or manual terminal commands.

Following the deployment of the global **Nothing OS Industrial Design System** (`industrial-nothing-os-design-system`), this change introduces a standalone Storage Explorer (`files.html`) that natively adopts the Industrial persona by default (with dot-matrix typography, Nothing Signal Red indicators, and technical partition cards) while supporting Bento mode, and strictly enforcing that `system_fs` is immutable read-only while `user_fs` is fully editable.

## What Changes

- **Prerequisite / Dependency**: Builds upon `industrial-nothing-os-design-system` (default Industrial style, `ndot57.woff2` font, Nothing Signal Red palette, and persona switcher).
- **Standalone Storage Explorer (`webui/files.html`)**:
  - Full-page file explorer built with DaisyUI 5 and Tailwind CSS.
  - Native **Industrial persona by default**: Dot-matrix partition labels (`[01] SYSTEM.STORAGE // 🔒 RO`, `[02] USER.STORAGE // ✏️ RW`), discrete dot progress meters, 16px technical dot-grid canvas, monospaced metadata, and Nothing Signal Red accent badges.
  - Automatically adapts to **Bento persona** with rounded squircle cards and soft sans-serif type if toggled in System Settings.
- **Strict Partition Security**:
  - `system_fs` is strictly **Read-Only** (allows browsing, downloading, file preview, and copying out to user storage; write/delete/format actions are forbidden and blocked by backend guards).
  - `user_fs` is fully **Read-Write** (allows file upload, directory creation, in-browser text/code/hex editing, renaming, deletion, and bulk download).
- **Filesystem REST API (`firmware-rs/src/rest.rs` & `fs_http.rs`)**:
  - `GET /api/v1/fs/stat`: Reports byte totals, used, and free space for both partitions.
  - `GET /api/v1/fs/list?root={system|user}&path={path}`: Enumerates directory entries with metadata.
  - `GET /api/v1/fs/read?root={system|user}&path={path}`: Streams file contents in 512-byte chunks.
  - `POST /api/v1/fs/write?root=user&path={path}`: Streams uploads directly to LittleFS (rejects with 403 on `system_fs`).
  - `DELETE /api/v1/fs/entry?root=user&path={path}`: Deletes file or directory (rejects with 403 on `system_fs`).
  - `POST /api/v1/fs/mkdir?root=user&path={path}`: Creates directory in `user_fs`.
  - `POST /api/v1/fs/rename?root=user&src={src}&dst={dst}`: Renames/moves files in `user_fs`.
  - `POST /api/v1/fs/copy?src_root={root}&src={src}&dst_root=user&dst={dst}`: Copies files into `user_fs`.
- **System WebUI Integration**:
  - HTTP server route `/files` and `/files.html` served via `picoserve` and `fs_http.rs`.
  - Added "Files" app tile in `webui/index.html` under System Instruments with dock pinning.
  - Added `files.html` to pre-compressed gzip packaging pipeline (`tools/pack_webui.py`).

## Capabilities

### New Capabilities
- `bento-file-manager`: Dedicated standalone `files.html` application with Industrial (default) and Bento personas, real-time byte telemetry for `system_fs` and `user_fs`, breadcrumb path navigation, responsive tree sidebar, grid / list views, in-place text/JSON file editor, and file action dispatching.
- `filesystem-rest-api`: HTTP REST endpoints (`/api/v1/fs/stat`, `/api/v1/fs/list`, `/api/v1/fs/read`, `/api/v1/fs/write`, `/api/v1/fs/delete`, `/api/v1/fs/mkdir`, `/api/v1/fs/rename`, `/api/v1/fs/copy`) with strict read-only enforcement for `system_fs` and full read-write execution for `user_fs`.

### Modified Capabilities
- `unified-os-app-launcher`: Register `files` / File Manager tile in the System Instruments grid and allow dock pinning.
- `web-desktop-daisyui-shell`: Add routing for `/files` and `/files.html` pointing to `files.html`, and bundle `files.html` into the gzip-packed WebUI asset pipeline.

## Impact

- **Frontend Assets**: New file `webui/files.html`, minor updates to `webui/index.html` (launcher tile) and `webui/app.js` (dock pinning/navigation).
- **Firmware Code**: `firmware-rs/src/rest.rs`, `firmware-rs/src/fs_http.rs`, and `firmware-rs/src/web.rs` for routing and filesystem handlers.
- **Zero-RAM Impact**: Chunked 512-byte streaming over `picoserve` maintains strict `#![no_std]` zero-heap and bare-metal memory limits.
