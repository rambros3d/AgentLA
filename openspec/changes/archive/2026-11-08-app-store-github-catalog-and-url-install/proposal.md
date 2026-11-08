## Why

Users need a seamless way to discover and install community applications into the AgentLA LittleFS partition directly from GitHub without manually compiling tarballs or downloading archives. By supporting GitHub repository catalog URLs and direct GitHub app folder URLs (e.g., `https://github.com/rambros3d/AgentLA/tree/embassy/apps/servo-master`), users can install tools with zero friction while keeping the microcontroller firmware completely free of TLS and DNS dependencies.

## What Changes

- **Default Catalog URL**: Point default catalog to `https://github.com/rambros3d/AgentLA/tree/embassy/apps/catalog.json` (normalized transparently to raw GitHub CDN).
- **GitHub URL Normalization**: Automatically convert `github.com/:owner/:repo/(blob|tree)/:branch/:path` URLs to CORS-friendly `raw.githubusercontent.com` URLs across both catalog and app fetches.
- **Convention-based Direct App URL Installation**: Fetch and bundle app folder contents (`app.json`, `index.html`, `icon.svg`, and any manifest-declared files) directly in the browser using an in-memory USTAR tarball builder, then stream to `POST /api/v1/apps/install/{name}`.
- **Enhanced Catalog Schema**: Update `apps/catalog.json` to support `icon_url` (with relative path resolution), `app_url`, hardware capability requirements, and app descriptions.
- **Store UI Controls**: Add an "Install from URL" action modal and an interactive repository source bar with change/reset controls in the App Store tab.

## Capabilities

### New Capabilities
- `github-raw-app-fetcher`: Browser-side convention resolver and packager that normalizes GitHub URLs, fetches app manifests and asset files from raw CDN endpoints, and builds multi-file USTAR tarballs for LittleFS installation.

### Modified Capabilities
- `community-app-store`: Adds direct GitHub app folder installation, URL normalization for catalogs and apps, enhanced catalog metadata schema, and store repository configuration controls.

## Impact

- **Affected Files**:
  - `webui/apps.html`: URL normalization logic, convention fetcher, multi-file USTAR tarball packager, "Install from URL" modal, and repository config bar.
  - `webui/style.css`: Styles for the repository bar, URL install modal, and store action buttons.
  - `apps/catalog.json`: Updated schema with `icon_url`, `app_url`, and relative asset paths.
  - `apps/servo-master/app.json` & `apps/can-sniffer/app.json`: Updated app manifests to match convention.
- **APIs**: Existing LittleFS REST API (`POST /api/v1/apps/install/{name}`, `GET /api/v1/apps/storage`) remains 100% untouched.
- **Firmware**: Zero changes required in `firmware-rs` or microcontroller crates; no increase in device flash or RAM footprint.
