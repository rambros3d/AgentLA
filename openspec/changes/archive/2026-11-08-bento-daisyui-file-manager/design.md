## Context

AgentLA implements a dual-partition LittleFS storage architecture:
1. `SYSTEM_FS` (512 KB, 128 sectors of 4 KB at `0x00204000`): Holds immutable WebUI static bundles (`index.html.gz`, `lab.html.gz`, `terminal.html.gz`, `daisyui.css.gz`, `style.css.gz`, `themes.css.gz`, fonts, Solar icon SVGs, CMSIS-Pack `.flm` flash algorithms, board manifests).
2. `USER_FS` (dynamically sized up to ~13.92 MB on BakuProbe / ~1.92 MB on Pico 2 at `0x00284000`): Stores installed micro-applications (`/apps/*`), user hardware profiles (`/profiles/*`), captured logic analyzer waveforms (`/captures/*.vcd`), and configuration logs.

Following the implementation of `industrial-nothing-os-design-system`, AgentLA possesses a global dual-persona engine. The File Manager in `webui/files.html` is built directly on this foundation, utilizing the **Nothing OS Industrial look as its default persona** (circular dot-matrix partition titles, 1px crisp outlines, 16px technical dot grid, Nothing Signal Red `#D71921` indicators, and monospaced metadata) while seamlessly supporting the **Bento persona** if selected in System Settings.

Strict partition safety is enforced: `system_fs` is permanent read-only, and `user_fs` is fully editable.

## Goals / Non-Goals

**Goals:**
- **Dedicated Application Page (`webui/files.html`)**: Clean, standalone Storage Explorer utilizing DaisyUI 5, Tailwind CSS, and Solar Icons.
- **Industrial-First Design**: Inherits the default `data-style="industrial"` Nothing OS look with NDot dot-matrix headings (`[01] SYSTEM.STORAGE //  RO`) and Nothing Signal Red accents, while gracefully falling back to Bento if toggled.
- **Physical Partition Transparency**: Real-time visualization of used and free storage for both `system_fs` and `user_fs` via DaisyUI `stats` and `progress` bars.
- **Strict Read-Only Enforcement on `system_fs`**: Allow browsing, downloading, file inspection, and copying *out* to user storage, but strictly prohibit any modification, upload, deletion, or renaming.
- **Full Read-Write Operations on `user_fs`**: Support file uploads, folder creation, renaming, deleting, recursive zip/tar downloads, and in-browser editing.
- **In-Browser Text/JSON Editor**: Embedded DaisyUI `modal` with syntax-aware monospace editor for rapid editing of `.json`, `.toml`, `.js`, `.html`, `.css`, and `.csv`.
- **Zero-RAM Streaming Filesystem REST API**: 512-byte chunked streaming endpoints in `firmware-rs` adhering to `#![no_std]` zero-heap constraints.
- **Pre-Compressed Gzip WebUI Asset**: `files.html` is gzipped into `files.html.gz` (~12–15 KB) during packaging, fitting easily inside `SYSTEM_FS`.

**Non-Goals:**
- Heavy client-side IDEs like Monaco or VS Code Web (a lightweight, responsive DaisyUI monospace editor handles embedded configuration needs with zero lag).
- Arbitrary code execution or server-side scripting on files.
- Allowing writes to firmware slots (`SLOT_A`, `SLOT_B`) or boot metadata through this filesystem manager.

## Decisions

### 1. Phased Dependency on `industrial-nothing-os-design-system`
- **Decision**: `bento-daisyui-file-manager` is scheduled for implementation **after** `industrial-nothing-os-design-system` has introduced the global Nothing theme, dot-matrix font, and persona switcher.
- **Rationale**: Allows `files.html` to immediately benefit from the dot-matrix typographic tokens and Nothing Signal Red styling without duplicated effort.

### 2. Dedicated `files.html` vs. Tab in `lab.html`
- **Decision**: Put the File Manager into a separate `webui/files.html` document with `/files` and `/files.html` routes.
- **Rationale**: Keeps `lab.html` focused on hardware test & measurement instruments without bloating DOM or colliding with real-time WebSocket packet processors. Follows the successful precedent set by `terminal.html`.

### 3. Dual-Layer Security Boundary for `system_fs`
- **Decision**: Enforce read-only guarantees on both frontend and backend:
  - Frontend: When `root == 'system'`, editing and mutating buttons are hidden/disabled; the editor displays an informational banner ("System ROM Asset — Read Only") and disables saving.
  - Backend (`firmware-rs`): Any mutating REST request (`POST /api/v1/fs/write`, `DELETE /api/v1/fs/entry`, `POST /api/v1/fs/mkdir`, `POST /api/v1/fs/rename`) targeting `root=system` is rejected immediately with HTTP 403 Forbidden.
- **Rationale**: Guarantees probe recovery and prevents bricking even if API endpoints are called directly via curl or malicious scripts.

### 4. Zero-Heap 512-Byte Chunked REST API
- **Decision**: Implement `/api/v1/fs/*` endpoints in `fs_http.rs` and `rest.rs` using `picoserve` chunked writers and LittleFS 512-byte buffer reads/writes via `with_system_fs` and `with_user_fs`.
- **Rationale**: Respects the bare-metal RP2350 microcontroller RAM budget without dynamic heap allocations.

## Risks / Trade-offs

- **[Risk] Large File Editing in Browser**: Loading large files (>500 KB) into a browser textarea could cause UI slowdown.
  - *Mitigation*: Limit in-browser textarea editing to text files under 256 KB. For larger files, provide a direct Download action.
- **[Risk] Concurrent File Operations and Flash Mutex**: LittleFS operations acquire `FLASH_MUTEX`. Long downloads could block concurrent telemetry requests.
  - *Mitigation*: Chunk reads into 512-byte increments with cooperative async yields between blocks via Embassy, preventing socket starvation.
- **[Risk] Browser Caching of Dynamic User Files**: Serving user-modified files under `/api/v1/fs/read` might lead to stale browser cache.
  - *Mitigation*: Send `Cache-Control: no-cache, no-store, must-revalidate` on user-edited file responses.
