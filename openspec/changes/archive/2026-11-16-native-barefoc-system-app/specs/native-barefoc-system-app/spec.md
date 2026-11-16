## ADDED Requirements

### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/barefoc.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving barefoc asset from SYSTEM_FS
- **WHEN** client requests `GET /barefoc.html` or `GET /motor`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/motor` to `SystemWebFile::new("barefoc.html", "text/html")` and provide HTTP redirects for `/foc` and `/barefoc`.

#### Scenario: Accessing canonical motor route
- **WHEN** user navigates to `/motor`
- **THEN** server responds with HTTP 200 and serves BareFOC Studio

#### Scenario: Accessing motor alias routes
- **WHEN** user navigates to `/foc` or `/barefoc`
- **THEN** server responds with HTTP 301 redirect to `/motor`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `Motor` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope` or `/terminal`
- **THEN** the top navigation rail displays a link to `Motor` (`/motor`)

#### Scenario: Active state styling on motor page
- **WHEN** user is viewing `/motor`
- **THEN** the Motor navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration
The system SHALL integrate the BareFOC Studio launcher into the macOS-style dock and desktop application drawer.

#### Scenario: Launching from dock
- **WHEN** user clicks the Motor icon in the glass dock
- **THEN** the system navigates directly to `/motor`

### Requirement: WebUI Packaging Inclusion
The build utility `tools/pack_webui.py` SHALL include `barefoc.html` when packaging `SYSTEM_FS` image and update tarballs.

#### Scenario: Generating system update tarball
- **WHEN** `pack_webui.py` builds the update tarball
- **THEN** `barefoc.html` and its pre-compressed `barefoc.html.gz` are embedded in the output archive
