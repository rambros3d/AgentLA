# Interactive Fallback WebUI Recovery

## Purpose
Provides a self-contained, In-ROM interactive WebUI recovery interface (`recovery.html`) served when `SYSTEM_FS` is unpopulated, formatted, or missing required page templates, enabling in-browser tarball flashing, 1-click GitHub restore, and instant navigation to the developer diagnostics console.

## Requirements

### Requirement: Self-Contained In-ROM Interactive Recovery Page
When a client requests `/home`, `/index.html`, `/lab.html`, or `/terminal.html` and `SYSTEM_FS` is unpopulated or missing the requested template, the firmware SHALL serve a self-contained In-ROM interactive WebUI recovery interface (`recovery.html`) with HTTP 200 OK.

#### Scenario: Navigating to WebUI on unpopulated partition
- **WHEN** client requests `http://10.10.10.10/` or `http://10.10.10.10/home` and `SYSTEM_FS` contains no `index.html`
- **THEN** server returns HTTP 200 OK with the interactive recovery interface embedded in Flash ROM.

### Requirement: In-Page WebUI Tarball Flashing
The recovery interface SHALL provide an interactive file picker and drag-and-drop zone allowing the user to select and stream a `webui.tar` archive directly to `POST /api/v1/system/webui/update` with live upload progress.

#### Scenario: Uploading WebUI tarball through recovery page
- **WHEN** user selects `webui.tar` and clicks upload
- **THEN** client JavaScript streams the binary to `/api/v1/system/webui/update`, displays upload progress, and reloads the page upon completion.

### Requirement: Browser-Mediated GitHub 1-Click Update
The recovery interface SHALL provide a 1-click update button that queries the latest release on GitHub, downloads `webui.tar`, and pipes it directly into the probe update endpoint.

#### Scenario: 1-Click install from GitHub
- **WHEN** user clicks "Install Latest from GitHub" while connected to the internet
- **THEN** client JavaScript fetches the latest release metadata, retrieves `webui.tar`, and uploads it to `/api/v1/system/webui/update`.

### Requirement: Direct Diagnostics Console Navigation
The recovery interface SHALL include a prominent high-contrast navigation button linking directly to `/device` (`Device Diagnostics & Developer Console`).

#### Scenario: Switching to developer diagnostics
- **WHEN** user clicks the diagnostics button
- **THEN** the browser navigates immediately to `http://10.10.10.10/device`.
