## Context

AgentLA runs a dual-core RP2350 microcontroller with LittleFS flash storage hosting web applications at `/apps/{name}/`. The web interface is served via Embassy Picoserve at `http://10.10.10.10`. Because the microcontroller operates in `#![no_std]` without a TLS/HTTPS stack or public DNS resolution, all internet communication (such as reaching GitHub) must be handled by the client's browser, which already possesses network connectivity.

Currently, the App Store tab reads a hardcoded catalog and only supports downloading pre-built `.tar` archives or uploading local files. Users want to install applications directly by pasting a GitHub repository catalog URL or an app directory URL (e.g. `https://github.com/rambros3d/AgentLA/tree/embassy/apps/servo-master`).

## Goals / Non-Goals

**Goals:**
- Provide client-side GitHub URL normalization that handles `blob`, `tree`, and raw URLs transparently with open CORS.
- Implement convention-based raw fetching for app folders (`app.json`, `index.html`, `icon.svg`, and manifest files) without hitting GitHub API rate limits.
- Implement client-side multi-file USTAR tarball generation in JavaScript so that any GitHub app directory is converted into a standard `.tar` stream for `POST /api/v1/apps/install/{name}`.
- Provide a responsive "Install from URL" modal and repository source indicator with change/reset options.
- Update `apps/catalog.json` schema with relative icon path resolution and `app_url` references.

**Non-Goals:**
- Modifying microcontroller firmware (`firmware-rs`) or backend REST endpoints (`/api/v1/apps/install`).
- Adding Git cloning or WebAssembly Git libraries to the browser.
- Authenticating with private GitHub repositories (all community catalog and app sources are public).

## Decisions

### 1. Convention-Based Raw Fetching over GitHub REST API
- **Decision**: Fetch raw files from `https://raw.githubusercontent.com/...` instead of using `https://api.github.com/repos/.../contents`.
- **Rationale**: `raw.githubusercontent.com` provides open CORS headers (`access-control-allow-origin: *`) and has no 60 requests/hour unauthenticated IP rate limit. It also avoids parsing nested GitHub API JSON trees, saving JavaScript bundle size in the WebUI partition.
- **Alternatives Considered**: GitHub Contents API (rate-limited to 60 req/hr per IP; requires fallbacks and larger JS bundle).

### 2. Client-Side Multi-File USTAR Builder
- **Decision**: Extend client-side packaging in `webui/apps.html` from single-file HTML to multi-file USTAR tarballs.
- **Rationale**: A TAR file format consists of 512-byte header blocks and 512-byte padded data blocks. Writing this in ~40 lines of pure vanilla JS eliminates any external JS dependencies (e.g. tar-js) and creates binary byte-for-byte compatibility with the RP2350 firmware's LittleFS unpacker.
- **Alternatives Considered**: Forcing developers to commit pre-built `.tar` files to GitHub (cumbersome, prone to merge conflicts, bad git hygiene).

### 3. Relative Icon URL Resolution
- **Decision**: Support both relative paths (`servo-master/icon.svg`) and absolute URLs (`https://...`) in `catalog.json`.
- **Rationale**: If the catalog is hosted at `https://raw.githubusercontent.com/user/repo/branch/apps/catalog.json`, a relative icon path `servo-master/icon.svg` automatically resolves to `https://raw.githubusercontent.com/user/repo/branch/apps/servo-master/icon.svg`. This allows the catalog to remain portable across forks and branches without hardcoding full URLs.

### 4. Offline-First Fallback
- **Decision**: Retain local `/apps/catalog.json` as the offline-first baseline.
- **Rationale**: If the user's PC is connected only to the local device network (e.g. `10.10.10.10`) without internet, the App Store remains functional and displays all built-in/seeded applications.

## Risks / Trade-offs

- **[Risk] GitHub branch or directory renaming**: If the branch or folder on GitHub is moved, raw fetching will return HTTP 404.
  - *Mitigation*: The UI provides clear error reporting ("Failed to fetch app.json at ...") and tests the URL before attempting tarball creation.
- **[Risk] Insufficient flash storage for downloaded app**: Downloading and streaming an app that exceeds free LittleFS space.
  - *Mitigation*: The "Install from URL" modal queries `/api/v1/apps/storage` before installation and blocks installation if estimated size exceeds available bytes.
- **[Risk] Browser CORS blocking on custom hosts**: If a user enters a custom non-GitHub URL that lacks CORS headers.
  - *Mitigation*: Clearly indicate supported URLs (GitHub repos and raw static hosts) in the modal helper text.
