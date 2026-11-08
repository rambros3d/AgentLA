## 1. Catalog Schema & App Manifest Updates

- [x] 1.1 Update `apps/catalog.json` with `icon_url`, `app_url`, relative icon resolution, and hardware capability tags.
- [x] 1.2 Verify and update `apps/servo-master/app.json` and `apps/can-sniffer/app.json` manifests to match the convention.

## 2. GitHub URL Normalizer & Convention Fetcher

- [x] 2.1 Implement `normalizeGithubUrl(url)` in `webui/apps.html` to transform tree/blob/raw URLs to `raw.githubusercontent.com`.
- [x] 2.2 Implement `resolveCatalogIconUrl(iconPath, catalogBaseUrl)` supporting both relative paths and absolute URLs with fallback icons.
- [x] 2.3 Implement `fetchAppFromUrl(url)` convention loader to fetch `app.json`, `index.html`, and `icon.svg` without hitting GitHub API rate limits.

## 3. Client-Side Multi-File USTAR Packager

- [x] 3.1 Implement `buildMultiFileTar(files)` in `webui/apps.html` to generate valid in-memory USTAR tarballs for multiple files.
- [x] 3.2 Wire `buildMultiFileTar` into the sheet installation handler so folder downloads stream directly to `POST /api/v1/apps/install/{name}`.

## 4. App Store UI & URL Installation Modal

- [x] 4.1 Add the Repository Source bar in `#view-appstore` showing the active catalog URL with edit and "Reset to Default" buttons.
- [x] 4.2 Implement the "Install from URL" modal with URL input, live preview card, hardware requirement check, and flash space verification.
- [x] 4.3 Add responsive styles in `webui/style.css` for the repository source bar, URL install modal, and store action buttons.

## 5. Verification & Packaging

- [x] 5.1 Package the WebUI using `python3 tools/pack_webui.py` and verify flash partition headroom.
- [x] 5.2 Upload `build/webui.tar` to `http://10.10.10.10/api/v1/system/webui/update` and verify live catalog loading, repo switching, and direct GitHub folder installation.
