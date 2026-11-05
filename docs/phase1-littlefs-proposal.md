# Phase 1 Proposal: LittleFS Mount, Picoserve File Serving & Tarball App Installer (Embassy Rust)

**Target branch:** `embassy` (worktree `/run/media/sun/halo/AgentLA-rust`)
**Scope crate:** `firmware-rs/`
**Reference implementation (C, `dev` branch):** `firmware/src/littlefs_driver.c`, `tar_unpacker.c`, `app_routing.c`, `seed_apps.c`, `rest_api.c`

> **Decisions locked for this phase** (override only if a blocking reason emerges):
> 1. LittleFS partition = **2 MB**, i.e. `PARTITION_SIZE_BYTES` grows from 1 MB. `LITTLEFS_START_OFFSET` stays `0x00190000` (profile slots at `1536*1024` sit *below* the partition and must not be touched).
> 2. **Full scope**: mount + static file serving from LittleFS + tarball app install + app list + storage-stats + recursive delete + seed default app.
> 3. Embedded static fallback page (`INDEX_HTML` / `STYLE_CSS` / `APP_JS` in `web.rs`) is **retained** as a safe fallback whenever LittleFS is unmounted or a file is missing. The device must always boot a working UI.

---

## 1. Goal

Bring the Rust port to parity with the C firmware's storage/web foundation:

- Mount the LittleFS partition at boot (`storage.rs` is already written, never called from `main.rs`).
- Serve static files from LittleFS through Picoserve, with MIME handling and streaming responses.
- Port the streaming ustar tarball unpacker so web `.tar` app bundles can be installed from a browser.
- Expose the app-ecosystem REST endpoints from the C firmware (`/api/v1/apps/**`).

## 2. Current state (verify, don't re-derive)

- `firmware-rs/src/storage.rs`: `FlashStorage` (block 4096, `READ_SIZE`=16, `WRITE_SIZE`=256), `init_filesystem()` with `mount_or_else(format)`, global `FS_MUTEX`. Unused today; unchanged constants point at 1 MB.
- `firmware-rs/src/main.rs`: spawns usb/net/dhcp/dns/web tasks + watchdog; never touches `p.FLASH`.
- `firmware-rs/src/web.rs`: embedded strings + Picoserve router (`/`, `/style.css`, `/app.js`, `/api/v1/system`, `.../bootsel`, `/api/v1/pins`, `/api/v1/programmer/**`, SSE `/api/v1/events` in `TelemetryEvents`).
- `firmware-rs/Cargo.toml`: `littlefs2 = { version = "0.8.1", default-features = false }`, `picoserve = 0.20.1`, `embassy-rp = 0.10.0` (rp235xa), target `thumbv8m.main-none-eabihf`.

## 3. Implementation plan

### 3.1 `storage.rs`
- Grow `PARTITION_SIZE_BYTES` to `2 * 1024 * 1024` (recompute `BLOCK_COUNT`).
- `init_filesystem()` must be called from `main.rs` **before** spawning the watchdog task. Rationale: a first-boot format erases 512 × 4 KB sectors and can take seconds; the watchdog is fed on 5 s windows and must not see the format.
- Return the mount result (already `bool`); thread it into the web task or let the web task check `FS_MUTEX` lazily — prefer **lazy check** (websites fall back to embedded when unmounted, no wiring needed).

### 3.2 New module `firmware-rs/src/apps.rs` — tar unpacker + app registry
Port `tar_unpacker.c` as a streaming state machine (do **not** buffer whole tarballs in RAM):

- States `Header → Data → Padding → Done|Error`; 512-byte ustar header buffer; octal size parse; per-file accumulation counters. Feed API: `feed(&mut [u8]) -> usize` (bytes consumed).
- Destination root always `/apps/{app_name}/…`. Path sanitization (security-critical):
  - reject empty name, `..` components, absolute paths, embedded `/apps/` prefix, backslashes, NULs, over-long names (ustar split `name` + `prefix` field must be handled).
  - all writes land under the app dir; `mkdir -p`-style nested dir creation is required.
- Execute all LittleFS ops while holding `FS_MUTEX` (littlefs2 is sync; blocking flash writes stall the core for page writes only — acceptable, watchdog headroom exists).
- Recursively remove an app dir (`DELETE`): littlefs2 may lack `remove_dir_all` — walk with `Dir` and depth-limit; if depth-limit is hit, fail with an error response.
- List apps (`GET /api/v1/apps`): enumerate `/apps/*` dirs, read each `app.json` (optional), emit JSON (use `heapless::String` / manual `write!`; no alloc).
- Storage stats (`GET /api/v1/apps/storage`): used/free from LittleFS (vfs stat or computed), emit JSON.
- Seed default app on fresh **format** (hint: track whether `mount_or_else` took the format path): port `seed_apps.c` minimally — `/apps/servo-master/app.json` + `index.html` (keep both tiny).

### 3.3 New module `firmware-rs/src/fs_http.rs` — serving files from LittleFS
- Implement a streaming response body for Picoserve (research the exact trait in `picoserve` 0.20.x: likely `WriteBody`/`response::Body` — read crate source/docs.rs; do **not** assume). Read 512–1024-byte chunks through `FS_MUTEX` and write them out.
- MIME map by extension: `html, css, js, json, svg, png, ico, gif, woff, woff2, txt` → Content-Type; unknown → `application/octet-stream` (do **not** inline-inject X-Content-Type-Options nesting here).
- Wildcard route: `/apps/{name}/{*path}` → `/apps/{name}/{path}` in LittleFS. If the file is missing → 404 JSON (`{"status":"not_found"}`).
- Keep `make_app()` in `web.rs` adding these routes; leave the three embedded static routes (`/`, `/style.css`, `/app.js`) untouched as fallback (do not redirect through LittleFS).

### 3.4 `web.rs` additions
- `POST /api/v1/apps/install` — streamed body. Research how Picoserve 0.20 exposes the request body stream (`picoserve::Request` extractor / `request.body()`); feed chunks (≤ 2 KB at a time) into the `apps.rs` unpacker. Enforce:
  - `app_name` from a query param or path segment (`/api/v1/apps/install/{name}` is also fine — pick whichever Picoserve handles more cleanly).
  - total cap 768 KB (well below the new 2 MB partition); on overflow abort, discard buffered state, return 413-style JSON error.
  - on success respond `{"status":"installed","app":"…"}` directly (the C firmware needed a separate `install_done` because of its HTTP pipeline; in Rust the single endpoint can respond immediately — no separate `install_done` endpoint required).
- `GET /api/v1/apps`, `GET /api/v1/apps/storage`, `GET /api/v1/apps/{name}/manifest`, `DELETE /api/v1/apps/{name}` as above, all JSON via the same tuple style already used in `web.rs` (`(("Content-Type","application/json"), body)`).

### 3.5 `main.rs` wiring
- `let flash = Flash::<…, Blocking, 4_194_304>::new(p.FLASH, irqs-config as needed);` then `storage::init_filesystem(flash);` placed **before** `spawner.spawn(watchdog_task(watchdog))`.
- `mod apps; mod fs_http;` declared. No other task restructuring.

## 4. Autonomous research checklist (the subagent is expected to look these up, not ask)

1. **picoserve 0.20.x**: exact streaming-response-body trait + how to wrap it; wildcard/path-segment routing syntax; request-body streaming access; body-size limit config. Sources: local `~/.cargo/registry/src/*/picoserve-0.20.1/`, docs.rs.
2. **littlefs2 0.8.x**: whether `default-features = false` breaks anything needed (esp. the `lfs2`-related feature if present); concrete `Dir`/`File` APIs for mkdir / recursive walk / stat; `Filesystem::format` cost assumptions.
3. **embassy-rp 0.10 Blocking Flash**: confirm IRQ safety guarantees while flash ops run (read `embassy-rp` flash driver source). If flash-op IRQ safety is worst than "interrupts disabled around the command sequence," document it and keep every individual flash op short (never hold `FS_MUTEX` across long multi-sector loops without yielding).
4. **RAM/binary budget**: after building, check `firmware-rs/target/thumbv8m.main-none-eabihf/release/firmware-rs` size against `memory.x` (FLASH size + RAM is 520 KB in comments; verify actual limits from `memory.x`). Report numbers at the end.

## 5. Out of scope (explicitly)

- Web-desktop DaisyUI UI port (docking/theming/icons), profile slots (`PROFILE_FLASH_ADDR` region), uploads >1 app at a time, WebSocket backup, any analyzer/adapter/programmer changes, Picoserve worker-count/keep-alive tuning beyond what makes install tests stable.

## 6. Acceptance / verification checklist

Run from `firmware-rs/`:

1. `cargo build --release` (target `thumbv8m.main-none-eabihf`) — clean build, no new warnings.
2. Flash to device (existing `probe-rs run --chip RP2350` runner in `.cargo/config.toml`) and over a fresh first boot confirm defmt prints `LittleFS partition mounted at 0x00190000` **or** the format-fallback warn line followed by a mount success.
3. `curl http://10.10.10.10/` still returns the embedded fallback page (device UI unchanged when no apps installed).
4. `curl http://10.10.10.10/api/v1/apps` → valid JSON (initially contains only the seeded `servo-master` entry).
5. Build a tiny ustar fixture locally (e.g. `tar cf /tmp/demo.tar index.html app.json` with a single `index.html/app.json`), then:
   `curl -T /tmp/demo.tar http://10.10.10.10/api/v1/apps/install/demo-app` → `{"status":"installed","app":"demo-app"}`.
6. `curl http://10.10.10.10/apps/demo-app/index.html` returns the uploaded file with a correct Content-Type.
7. `curl http://10.10.10.10/api/v1/apps/storage` reports plausible used/free numbers.
8. `curl -X DELETE http://10.10.10.10/api/v1/apps/demo-app` then re-`GET /api/v1/apps` → entry gone.
9. Re-read `/api/v1/system` and `/api/v1/events` — SSE/telemetry still work; UI embedded routes still work.
10. Unplug/replug/dev-reboot → installed app persists across reboot (flags the LittleFS mount actually survived a `mount` without format).

## 7. Known risk register

| Risk | Likelihood | Mitigation |
|---|---|---|
| Picoserve streaming-body / wildcard API differs from assumption | High (API drift across 0.x) | Research section §4 item 1; adjust code to real API, not assumptions |
| littlefs2 feature set (`default-features=false`) breaks needed APIs | Medium | Add back only the specific feature(s) needed; re-check `cargo tree` |
| Flash erase IRQ-side effects during format on first boot | Medium (verify driver; watchdog no longer runs during mount) | Mount before watchdog spawn; keep subsequent writes page-sized |
| Upload request body stalls and pins `FS_MUTEX` (SSE starved) | Medium | Hard cap on total bytes; chunked reads with short ops; document worst-case block durations in defmt |
| Recursive delete misses state on malformed tar | Low | Strict path sanitization up front; reject before any I/O |
