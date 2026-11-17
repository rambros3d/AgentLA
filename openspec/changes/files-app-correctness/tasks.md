## 1. Firmware: trustworthy directory listings

- [x] 1.1 Spike picoserve `ChunkedResponse`/`Chunks` against littlefs2's `read_dir_and_then` to confirm whether a directory iterator can be emitted incrementally across `await`; record the decision in `design.md`.
- [x] 1.2 Replace the fixed 2048-byte `serialize_dir_entries` buffer with streaming JSON emission (preferred) or an 8 KB buffer plus explicit `500 {"error":"listing_too_large"}` contingency; never return a truncated 200.
- [x] 1.3 Add a JSON string escaper in `firmware-rs/src/fs_http.rs` and apply it to directory entry names in `FsList`.
- [x] 1.4 Verify `GET /api/v1/fs/list` returns valid JSON for a directory with more entries than the old 2048-byte limit.

## 2. Firmware: atomic writes and safe responses

- [x] 2.1 Apply the JSON escaper to the path/name echoes in `FsWrite`, `FsEntry`, `FsMkdir`, and `FsRename` success responses.
- [x] 2.2 Rework `FsWrite` to stage the full payload to a temporary path and atomically rename into place, removing the staged file on any chunk error.
- [x] 2.3 Add a `503 {"error":"fs_busy"}` path for `try_lock` contention in `FsStat`/`FsList`/`FsRead`/`FsWrite`/`FsEntry`/`FsMkdir`/`FsRename`, distinct from `fs_not_mounted`.
- [x] 2.4 Add host-side unit tests for the JSON escaper, including a filename containing a double quote and a backslash.

## 3. Client: editor boundary and listing feedback

- [x] 3.1 Gate `onEntryClick`/`openFileInEditor` on the editable extension allow-list and 256 KB size cap; show an explicit message for rejected files.
- [x] 3.2 Rework `fetchDirectory` to surface an explicit error state on malformed/non-array/non-OK responses and to stop substituting mock entries when running on device firmware.
- [x] 3.3 Replace filename/path interpolation in `renderEntries` with `textContent` and `addEventListener` handlers so uploaded filenames cannot break out of inline `onclick` attributes.

## 4. Client: dead code and single theme authority

- [x] 4.1 Remove the unused `isDemo` flag, the phantom `btn-part-user`/`btn-part-system` lookups, `toggleFilesTheme`, and the duplicate local `storage`/`BroadcastChannel` theme listeners from `webui/files.html`.
- [x] 4.2 Verify `#btn-theme-toggle` cycles the theme exactly once through `agentla-bridge.js` and that `style.colorScheme` is updated.

## 5. Verification

- [x] 5.1 Build firmware and confirm the crate remains `#![no_std]` with no heap dependency added.
- [ ] 5.2 On-device check: large directory renders completely (or errors explicitly), never shows a false empty state.
- [ ] 5.3 On-device check: `.gz`/font/FLM assets are refused by the editor; oversized text files are refused.
- [ ] 5.4 On-device check: an interrupted write leaves the original config file intact.
- [ ] 5.5 On-device check: a file whose name contains a quote round-trips through listing, editor, rename, and delete with valid JSON.

## 6. Spec alignment

- [ ] 6.1 During sync/archive, update the `bento-file-manager` main spec Purpose and requirement headings to the Industrial Storage Console / Nothing UI wording, leaving the capability directory name unchanged.
