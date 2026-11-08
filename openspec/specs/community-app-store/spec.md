# community-app-store Specification

## Purpose
Community app catalog discovery, offline fallback caching, one-click installation, and update detection for the AgentLA web desktop.
## Requirements
### Requirement: Remote Community Catalog Fetching
The App Launcher web interface SHALL fetch and parse an external `catalog.json` catalog directly from the user's browser without requiring TLS or internet connectivity on the RP2350 microcontroller. The default catalog repository URL SHALL be `https://github.com/rambros3d/AgentLA/tree/embassy/apps/catalog.json`, automatically normalized to raw GitHub CDN endpoint.

#### Scenario: Loading the community store tab
- **WHEN** user selects the "App Store" tab in `http://10.10.10.10/apps`
- **THEN** the browser checks local LittleFS `/apps/catalog.json` first, then fetches the configured GitHub catalog URL, normalizes it to raw CDN, parses app listings, and renders cards showing app icon, title, author, version, description, and tags.

#### Scenario: Offline catalog failure handling
- **WHEN** user opens the App Store tab while the host machine has no internet access
- **THEN** the UI loads the pre-bundled LittleFS apps catalog and displays an offline indicator with options to install apps via local file drag-and-drop or manual package upload.

### Requirement: App Version and Update Detection
The App Store interface SHALL cross-reference the catalog against installed apps returned from `/api/v1/apps` and indicate update availability.

#### Scenario: App already installed with matching version
- **WHEN** an app in the catalog has the same ID and version as an installed app
- **THEN** the card displays an "Installed" badge and a "Launch" button linking to `/apps/{app-name}/`.

#### Scenario: New version available
- **WHEN** an app in the catalog has a higher version number than the installed app
- **THEN** the card displays an "Update (vX.Y.Z)" button.

### Requirement: Configurable Registry and Sideload URLs
The App Store interface SHALL permit users to change the default catalog URL or enter direct download links for custom archives, GitHub app directories, and remote catalogs.

#### Scenario: Adding a custom catalog URL
- **WHEN** user enters a custom catalog URL in the store repository bar
- **THEN** the setting is saved to `localStorage` under `agentla_catalog_url`, the store re-renders using the custom catalog, and a "Reset to Default" button is available to restore the official repository.

#### Scenario: Sideloading from direct URL
- **WHEN** user pastes a direct download link to a `.tar` package or `.html` dashboard into the "Install from URL" modal
- **THEN** the browser fetches the archive, queries `/api/v1/apps/storage` for free flash space, and streams it to `/api/v1/apps/install/{name}` upon confirmation.

### Requirement: Direct GitHub App Directory Installation
The App Store interface SHALL allow installing applications by providing a GitHub folder URL (e.g. `https://github.com/rambros3d/AgentLA/tree/embassy/apps/servo-master`).

#### Scenario: Installing from GitHub tree folder URL
- **WHEN** user inputs a GitHub directory URL into the "Install from URL" dialog
- **THEN** the browser resolves the URL to raw GitHub CDN, fetches `app.json`, `index.html`, and `icon.svg`, verifies flash capacity, bundles the assets into a client-side USTAR tarball, and streams it to `/api/v1/apps/install/{app_name}`.

#### Scenario: Previewing app metadata before install
- **WHEN** user pastes an app directory or tarball URL into the "Install from URL" input
- **THEN** the modal dynamically fetches and displays the app name, version, author, description, and required hardware capabilities before prompting to install.

### Requirement: Relative Icon and Base URL Resolution
The catalog renderer SHALL resolve relative `icon_url` paths against the base directory of the active catalog URL.

#### Scenario: Relative icon path in catalog
- **WHEN** an app card specifies a relative icon path such as `servo-master/icon.svg`
- **THEN** the browser computes the full URL relative to the catalog source (e.g. `https://raw.githubusercontent.com/rambros3d/AgentLA/embassy/apps/servo-master/icon.svg`) and renders it with a fallback solar icon if the network fetch fails.

### Requirement: Neopixel Control App Catalog Registration
The official app catalog `apps/catalog.json` and packaging pipeline SHALL register the `neopixel-control` application with required metadata, tags, and dependencies.

#### Scenario: Discovering Neopixel Control in App Store
- **WHEN** user views the App Store catalog
- **THEN** `neopixel-control` appears with title "Neopixel Control", category "Lighting", icon `icon.svg`, and a link to install or launch


