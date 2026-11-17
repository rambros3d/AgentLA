## Why

A gap audit of the Files app found a silent, data-dependent failure: `/api/v1/fs/list` serialises directory entries into a fixed 2048-byte buffer and discards overflow, so a sufficiently large directory returns truncated, invalid JSON that the client swallows and renders as "Directory is empty". The same audit found the in-place editor opening binary/gzipped assets as text, no enforcement of the spec's 256 KB edit cap, unescaped filenames in both JSON responses and DOM handlers, and dead theme/session code.

Separately, the capability spec still describes a "Bento Storage Explorer" even though `industrial-design-system` declares the Nothing UI the sole permanent presentation and Bento retired. The shipped `files.html` already uses `data-theme="nothing"` / `data-style="industrial"`, so the spec is the thing that is out of date. Correcting the defects while aligning the spec wording keeps the contract honest and prevents silent data loss in the storage console.

## What Changes

- Make directory listing complete and trustworthy: never emit truncated JSON; report an explicit error rather than a 200 with a partial payload.
- Scope the in-place editor to the text extensions named in the spec and enforce the 256 KB limit on both open and save paths; refuse binary, gzip, and font assets with a clear message.
- Escape filenames and paths when building JSON responses and when interpolating into the file listing DOM, so uploaded filenames cannot corrupt payloads or break out of inline handlers.
- Remove dead client code (unused demo flag, phantom partition-button IDs, superseded local theme toggle) and consolidate theme handling on the bridge's `cycleThemeMode` path.
- Make `user_fs` writes atomic (staged write then rename) so a failed chunk cannot leave a truncated config or profile file.
- Make FS endpoint failures under mutex contention distinguishable from an unmounted filesystem.
- Rewrite the `bento-file-manager` requirement text to describe the Industrial Storage Console on the Nothing UI; **keep the capability directory name unchanged** to avoid breaking cross-references, despite the historical name.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `bento-file-manager`: replace Bento framing with the Nothing/Industrial presentation already in production; add explicit editor extension and size constraints; require complete directory listings; remove duplicate theme logic.
- `filesystem-rest-api`: require complete JSON listing responses, escaped string values, atomic user-file writes, and a distinct error for partition-lock contention.

## Impact

- `webui/files.html` — editor gating, escaping, dead-code removal, status messaging.
- `firmware-rs/src/fs_http.rs` — listing buffer handling, JSON escaping, atomic write path, contention response.
- `firmware-rs/src/storage.rs` — lock-acquisition semantics if contention handling changes.
- `openspec/specs/bento-file-manager/spec.md` and `openspec/specs/filesystem-rest-api/spec.md` — delta specifications from this change.
- No new REST endpoints, no flash-layout changes, no new crates. All firmware work stays `#![no_std]` and zero-heap.
