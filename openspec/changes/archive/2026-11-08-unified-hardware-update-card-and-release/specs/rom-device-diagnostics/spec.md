## ADDED Requirements

### Requirement: Unified System Update Hero Card in Hardware View
The In-ROM diagnostics console (`device.html`) SHALL render a full-width System Update Hero Card (`#cardSystemUpdate`) positioned directly at the top of the `[ HARDWARE ]` view panel. The card SHALL present a unified release management interface combining firmware dual-slot status, WebUI asset status, GitHub release version comparison, a single-click 2-phase full system update runner, manual local file staging pickers, and firmware rollback.

#### Scenario: Displaying unified system update card on hardware tab
- **WHEN** the user opens `http://<host>/device`
- **THEN** the console displays `#cardSystemUpdate` at the top of the Hardware panel displaying installed vs latest versions for both Firmware and WebUI alongside a unified `[ AUTO UPDATE ALL ]` button.

#### Scenario: Handling unpopulated WebUI state without separate recovery card
- **WHEN** `SYSTEM_FS` is blank, unformatted, or contains 0 used bytes
- **THEN** the unified card cleanly displays `WebUI: Not Installed` and provides 1-click cloud restoration (`Auto Update`) and local drag-and-drop `.tar` staging, with no secondary `#recoveryBanner` rendered.

### Requirement: Dedicated Storage Tab and Subpath Routing
The In-ROM diagnostics console (`device.html`) SHALL provide a dedicated `[ STORAGE ]` tab (`#tabStorage`) and view panel (`#viewStorage`) positioned immediately adjacent to `[ HARDWARE ]`. The storage panel SHALL host the `WebUI & Storage (SYSTEM_FS)` card, `User Storage (USER_FS)` card, and the full-width Flash Partitions linear memory bar and geometry table. The HTTP router SHALL register `/device/storage` and `/device/storage/` routing directly to this view.

#### Scenario: Navigating to /device/storage
- **WHEN** user navigates to `http://<host>/device/storage` or clicks the `[ STORAGE ]` tab
- **THEN** the browser opens the Storage view panel displaying the LittleFS partition cards, in-place partition formatting controls, and the flash geometry table without showing MCU silicon or telemetry cards.

## MODIFIED Requirements

### Requirement: In-Browser Update Checker with Individual Flashers
The diagnostics page SHALL provide an in-browser update checker that queries the GitHub Releases API (`api.github.com/repos/rambros3d/AgentLA/releases/latest`) using the client's internet connection, compares installed versions against remote releases, and provides both an atomic full-system 2-phase update and distinct, separate update actions for Firmware and WebUI.

#### Scenario: Detecting update availability from official repository
- **WHEN** the user opens the diagnostics page while connected to the internet
- **THEN** client JavaScript queries `https://api.github.com/repos/rambros3d/AgentLA/releases/latest`, compares the installed firmware version (from `/api/v1/system/firmware/status`) and WebUI asset manifest against the latest GitHub release, and updates the release tag badge and update status.

#### Scenario: Individual firmware update
- **WHEN** the user initiates a firmware update via 1-click download or file picker
- **THEN** the client streams the firmware binary to `POST /api/v1/system/firmware/ota`, displays upload progress, and prompts to reboot into the candidate slot.

#### Scenario: Individual WebUI update
- **WHEN** the user initiates a WebUI update via 1-click download or file picker
- **THEN** the client streams the update tarball to `POST /api/v1/system/webui/update`, displays extraction progress, and updates `SYSTEM_FS` without requiring a firmware reboot.

## REMOVED Requirements

### Requirement: Dynamic Recovery Banner above Tab Bar
**Reason**: Replaced by the unified Hero Card positioned inside the `[ HARDWARE ]` view panel.
**Migration**: All recovery capabilities (1-click cloud restore and local tarball upload) are now natively hosted within the System Update Hero Card, eliminating `#recoveryBanner` and duplicate `#rec*` elements.
