## MODIFIED Requirements

### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/barefoc.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving barefoc asset from SYSTEM_FS
- **WHEN** client requests `GET /barefoc.html` or `GET /barefoc`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/barefoc` to `SystemWebFile::new("barefoc.html", "text/html")` as the canonical route and provide HTTP 301 redirects for `/motor`, `/motor/`, `/foc`, `/foc/`, and `/barefoc/` to `/barefoc`.

#### Scenario: Accessing canonical barefoc route
- **WHEN** user navigates to `/barefoc`
- **THEN** server responds with HTTP 200 and serves BareFOC Studio

#### Scenario: Accessing alias routes
- **WHEN** user navigates to `/motor`, `/foc`, or `/barefoc/`
- **THEN** server responds with HTTP 301 redirect to `/barefoc`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `BareFOC` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope` or `/terminal`
- **THEN** the top navigation rail displays a link to `BareFOC` (`/barefoc`)

#### Scenario: Active state styling on barefoc page
- **WHEN** user is viewing `/barefoc`
- **THEN** the BareFOC navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration
The system SHALL integrate the BareFOC Studio launcher into the macOS-style dock and desktop application drawer.

#### Scenario: Launching from dock
- **WHEN** user clicks the BareFOC icon in the glass dock
- **THEN** the system navigates directly to `/barefoc`

### Requirement: ODrive-GUI-Style Studio Shell
The system SHALL present BareFOC Studio with a shell modeled on the ODrive GUI: a header of named dashboards with a menu for dashboard and target-configuration import/export and a global sampling toggle, a dynamic central view area for dashboards/wizard/start pages, and a persistent per-axis status footer.

#### Scenario: Studio shell layout
- **WHEN** user opens `/barefoc`
- **THEN** studio renders the dashboard header, active view, connection/safety toolbar, and persistent axis footer
