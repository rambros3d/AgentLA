## ADDED Requirements

### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/servo.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving servo asset from SYSTEM_FS
- **WHEN** client requests `GET /servo.html` or `GET /servo`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/servo` to `SystemWebFile::new("servo.html", "text/html")` and provide an HTTP redirect for `/servomaster`.

#### Scenario: Accessing canonical servo route
- **WHEN** user navigates to `/servo`
- **THEN** server responds with HTTP 200 and serves Servo Master

#### Scenario: Accessing servo alias route
- **WHEN** user navigates to `/servomaster`
- **THEN** server responds with HTTP 301 redirect to `/servo`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `Servo` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope`, `/terminal`, or `/motor`
- **THEN** the top navigation rail displays a link to `Servo` (`/servo`)

#### Scenario: Active state styling on servo page
- **WHEN** user is viewing `/servo`
- **THEN** the Servo navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration
The system SHALL integrate the Servo Master launcher into the floating glass dock and application launcher drawer.

#### Scenario: Launching from dock
- **WHEN** user clicks the Servo icon in the floating dock
- **THEN** the system navigates directly to `/servo`

### Requirement: Emergency Safe State Integration
The system SHALL detach all active PWM channels and release pin ownership when the emergency safe state is triggered.

#### Scenario: Safe state triggered from UI or hotkey
- **WHEN** user clicks the `SAFE STATE` button or presses `Shift+Esc`
- **THEN** the application halts all pulse generation and releases all claimed servo GPIO pins to High-Z

### Requirement: WebUI Packaging Inclusion
The build utility `tools/pack_webui.py` SHALL include `servo.html` when packaging the `SYSTEM_FS` image and update tarballs.

#### Scenario: Packaging SYSTEM_FS
- **WHEN** `python3 tools/pack_webui.py` is executed
- **THEN** `servo.html` is validated and packed into `build/webui.img` and `build/webui.tar`
