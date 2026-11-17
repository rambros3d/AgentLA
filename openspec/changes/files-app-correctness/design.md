## Context

The Files app is a single self-contained page (`webui/files.html`) backed by seven Picoserve handlers in `firmware-rs/src/fs_http.rs` and two LittleFS partitions managed by `firmware-rs/src/storage.rs`. An audit established four classes of defect: a truncated-listing failure that silently presents a populated directory as empty, an editor that accepts binary assets and has no size guard, unescaped filesystem-derived strings in both JSON and DOM contexts, and dead client code. The shipped UI already targets the Nothing presentation (`data-theme="nothing"`, `data-style="industrial"`) declared as permanent by `industrial-design-system`; only stale spec wording still says "Bento".

Constraints inherited from the project: all firmware code stays `#![no_std]` and zero-heap, partition boundaries are immutable at runtime, and no new crates or REST endpoints are introduced.

## Goals / Non-Goals

**Goals:**
- Guarantee that a directory listing is either complete or an explicit error, never a truncated success.
- Restrict the editor to the documented text extensions and size cap, on both open and save.
- Make filesystem-derived strings safe in JSON responses and in the listing DOM.
- Make `user_fs` writes atomic and distinguish lock contention from an unmounted partition.
- Align the `bento-file-manager` requirements with the Nothing/Industrial UI already in production.

**Non-Goals:**
- Renaming the capability directory or changing the REST URL surface.
- Reworking flash partition offsets, sizes, or the read-only boundary on `system_fs`.
- Reconciling other stale Bento-flavoured specs (`daisyui-bento-theme-system`, `bench-theme-engine`, `unified-os-app-launcher`, `persistent-iframe-app-shell`) — handled in a separate change.
- Adding pagination, search, or sorting as new user-facing features.

## Decisions

**1. Listing completeness over a bigger magic number.**
`serialize_dir_entries` currently uses a fixed `heapless::String<2048>` and ignores every overflow. Preferred fix: emit the array incrementally through the existing `ChunkedResponse` machinery (the same pattern used by `SystemFileChunks`) so no fixed ceiling exists. Contingency, if littlefs2's closure-based `read_dir_and_then` makes incremental emission impractical: raise the buffer to 8 KB and return `500 {"error":"listing_too_large"}` when even that is exhausted. In both cases a truncated 200 is forbidden. Rationale: silence is the actual bug; a visible error is recoverable, a fake-empty directory is not.

**1a. Spike result (task 1.1): contingency path selected.**
Inspected littlefs2 0.8.1 and picoserve 0.20.1 sources. `Filesystem::read_dir_and_then` accepts only a **synchronous** `FnOnce(&mut ReadDir) -> Result<R>` closure; the iterator's backing `ReadDirAllocation` and `ReadDir` live only for the duration of that call. The only way to hold a directory iterator across the `await` in `Chunks::write_chunks` is littlefs2's `unsafe fn read_dir`, which the crate itself warns "can induce UB just like File::open". Introducing unsafe filesystem iteration solely to stream a JSON array is not an acceptable trade in this `#![no_std]` codebase, and re-opening the directory once per chunk would turn an O(n) walk into O(n^2) flash reads. Decision: implement the **contingency** — a single 8 KB `heapless::String` response buffer with checked writes; any overflow returns `500 {"error":"listing_too_large"}` and never a truncated 200. This is exactly the behaviour required by the `filesystem-rest-api` spec scenario "Listing exceeds response capacity".

**2. Client-side gating plus server-side truth.**
The editor allow-list (`.json`, `.toml`, `.js`, `.html`, `.css`, `.txt`, `.csv`) and 256 KB cap are enforced in `files.html` before content is fetched and before save is posted. `FsRead` remains a generic byte stream (it must also serve downloads); the boundary is a UI contract, not a server MIME restriction. Rejected files produce an explicit toast, never a garbled textarea. A failed gzip decode was rejected as an alternative because `SystemFileChunks` deliberately streams raw bytes and `Content-Encoding` negotiation belongs to `SystemWebFile`.

**3. Escape at the boundary in both directions.**
Server: route every filename/path through a small JSON string escaper before `write!`. Client: stop interpolating names into `innerHTML`/inline `onclick`; build elements with `textContent` and attach listeners via `addEventListener` using captured values. Rationale: both surfaces receive attacker-influenced filenames from uploads, and neither currently escapes.

**4. Atomic user-file writes.**
Stage the full request body to a temporary path under the same parent directory, then `fs.rename` to the target. On any chunk error, remove the temporary file and leave the original untouched. Rationale: the editor and profile files are JSON; a half-written JSON file is worse than a failed save.

**5. Single theme authority.**
Delete `toggleFilesTheme` and the local `storage`/`BroadcastChannel` theme listeners from `files.html`; `agentla-bridge.js` (`cycleThemeMode` / `syncTheme`) is the sole owner. Rationale: two listeners with divergent implementations (only one sets `style.colorScheme`) produce inconsistent state.

**6. Spec wording, not spec identity.**
Keep the capability directory `bento-file-manager` and rename only the affected requirement heading to "Standalone Industrial Storage Console Page". Rewriting the main spec Purpose text is deferred to the sync/archive step. Rationale: directory renames break cross-references in launcher and telemetry specs that are explicitly out of scope here.

## Risks / Trade-offs

- [Chunked directory emission holds the FS lock longer] → Keep each chunk small and release between entries; if the async/closure boundary cannot be crossed, use the 8 KB buffer + explicit error contingency.
- [Atomic write doubles flash writes temporarily] → LittleFS wear is bounded by the staged-file lifetime; cleanup runs on every error path. Acceptable for the low write volume of this UI.
- [Removing mock fallback makes demo mode noisier] → Demo mode currently relies on failed fetches; the new error state is explicit, which is preferable to a false "empty directory".
- [Escaping refactor touches every row render] → Limit the change to `renderEntries`; keep grid/table markup otherwise unchanged to bound regression risk.
- [Nothing UI wording diverges from other stale specs] → Documented as out of scope; the authoritative `industrial-design-system` already resolves the conflict.

## Migration Plan

1. Firmware handlers first (`fs_http.rs`), so the client has an honest API to target.
2. Client gating, escaping, and dead-code removal in `files.html`.
3. Main spec text updated when this change is synced/archived; no device-side migration needed.
4. Rollback is a normal firmware rollback; no on-disk format changes are made.

## Open Questions

- Does picoserve's `Chunks` trait permit holding a littlefs2 directory iterator across `await`, or must entries be re-opened per chunk? **Resolved (task 1.1):** only via littlefs2's unsafe `read_dir`; rejected in favour of the 8 KB buffer + explicit error contingency (decision 1a).
- Should oversized listings return `500 listing_too_large` or introduce a `413`-style status available in this picoserve version? **Resolved:** the main spec explicitly requires `500 {"error":"listing_too_large"}`, so no new status code is introduced.
- Should the staged-write temp path be fixed (e.g. `.<name>.tmp`) or generated from a monotonic counter to avoid collisions with concurrent saves? **Resolved:** a fixed per-target `.<name>.tmp` sibling path is used. Two concurrent saves of the *same* target may share the temp file (last writer wins), but the original target is never truncated until a successful atomic rename; this keeps the change zero-heap and deterministic.
