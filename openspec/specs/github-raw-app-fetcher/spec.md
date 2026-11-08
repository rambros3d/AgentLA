# github-raw-app-fetcher Specification

## Purpose
TBD - created by archiving change app-store-github-catalog-and-url-install. Update Purpose after archive.
## Requirements
### Requirement: GitHub URL Normalization
The resolver SHALL accept diverse GitHub URL formats (`tree`, `blob`, or raw) and convert them to canonical `raw.githubusercontent.com` URLs with CORS support.

#### Scenario: Normalizing a GitHub blob catalog URL
- **WHEN** user provides `https://github.com/rambros3d/AgentLA/blob/embassy/apps/catalog.json`
- **THEN** the resolver transforms it to `https://raw.githubusercontent.com/rambros3d/AgentLA/embassy/apps/catalog.json`.

#### Scenario: Normalizing a GitHub tree catalog URL
- **WHEN** user provides `https://github.com/rambros3d/AgentLA/tree/embassy/apps/catalog.json`
- **THEN** the resolver transforms it to `https://raw.githubusercontent.com/rambros3d/AgentLA/embassy/apps/catalog.json`.

#### Scenario: Normalizing a GitHub app folder URL
- **WHEN** user provides `https://github.com/rambros3d/AgentLA/tree/embassy/apps/servo-master`
- **THEN** the resolver identifies the base directory as `https://raw.githubusercontent.com/rambros3d/AgentLA/embassy/apps/servo-master/`.

### Requirement: Convention-Based App Directory Fetching
The fetcher SHALL retrieve application assets from the resolved raw directory based on `app.json` conventions without invoking rate-limited GitHub APIs.

#### Scenario: Fetching standard app assets
- **WHEN** an app directory is targeted for installation
- **THEN** the fetcher downloads `app.json`, parses the manifest, and downloads `index.html` and `icon.svg` (or any additional files listed in `app.json["files"]`).

#### Scenario: Handling missing manifest
- **WHEN** an app directory URL does not contain a valid `app.json`
- **THEN** the fetcher throws an informative error stating that the application manifest could not be found at the target URL.

### Requirement: Client-Side Multi-File USTAR Tarball Packaging
The packager SHALL bundle multiple file buffers into a compliant POSIX USTAR archive in browser memory.

#### Scenario: Bundling app files into USTAR tar
- **WHEN** `app.json`, `index.html`, and `icon.svg` buffers are fetched
- **THEN** the packager generates 512-byte USTAR header blocks with octal permissions, file sizes, and checksums, appends zero-padded file bodies, and appends two 512-byte end-of-archive blocks.

#### Scenario: Validating output tarball
- **WHEN** the generated tarball is streamed to `/api/v1/apps/install/{name}`
- **THEN** the LittleFS unpacker on the RP2350 successfully extracts `/apps/{name}/app.json`, `/apps/{name}/index.html`, and `/apps/{name}/icon.svg`.

