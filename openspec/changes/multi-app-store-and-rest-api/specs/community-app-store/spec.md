## ADDED Requirements

### Requirement: Remote Community Catalog Fetching
The App Launcher web interface SHALL fetch and parse an external `apps.json` catalog directly from the user's browser without requiring TLS or internet connectivity on the RP2350 microcontroller.

#### Scenario: Loading the community store tab
- **WHEN** user selects the "App Store" tab in `http://10.10.10.10/apps`
- **THEN** the browser fetches the configured catalog URL, parses the app listings, and renders cards showing app icon, title, author, version, description, and tags.

#### Scenario: Offline catalog failure handling
- **WHEN** user opens the App Store tab while the host machine has no internet access
- **THEN** the UI displays an informative offline state with an option to manually install apps via local file drag-and-drop.

### Requirement: App Version and Update Detection
The App Store interface SHALL cross-reference the catalog against installed apps returned from `/api/v1/apps` and indicate update availability.

#### Scenario: App already installed with matching version
- **WHEN** an app in the catalog has the same ID and version as an installed app
- **THEN** the card displays an "Installed" badge and a "Launch" button linking to `/apps/{app-name}/`.

#### Scenario: New version available
- **WHEN** an app in the catalog has a higher version number than the installed app
- **THEN** the card displays an "Update (vX.Y.Z)" button.

### Requirement: Configurable Registry and Sideload URLs
The App Store interface SHALL permit users to change the default catalog URL or enter direct download links for custom archives.

#### Scenario: Adding a custom catalog URL
- **WHEN** user enters a custom catalog URL in store settings
- **THEN** the setting is saved to `localStorage` and the store re-renders using the custom catalog.

#### Scenario: Sideloading from direct URL
- **WHEN** user pastes a direct download URL to a `.tar` or `.html` file into the sideload box
- **THEN** the browser fetches the archive, triggers the pre-install confirmation modal, and streams it to the device upon user confirmation.
