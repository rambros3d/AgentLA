# rom-device-diagnostics Specification

## Purpose
Provides a standalone, self-contained Flash ROM diagnostics and recovery interface (`device.html`) on `/device` and `/device.html`. The page operates with zero dependencies on LittleFS partitions, presenting live board telemetry, dual-slot A/B status, a browser-mediated update checker with independent firmware and WebUI flashers, a visual Flash partition map, dedicated `SYSTEM_FS` and `USER_FS` storage controls, and an in-page Neo-Industrial confirmation modal for developer recovery tools (hands-free BOOTSEL reboot, pin arbiter emergency releases, pin probing, and partition formatting).

## Requirements

### Requirement: ROM-Embedded Standalone Diagnostics Page
The system SHALL embed the complete HTML, CSS, and JavaScript source of `device.html` directly into the firmware Flash binary via `include_str!` or `include_bytes!`, serving it with `Content-Type: text/html` and `Cache-Control: no-cache, must-revalidate` without accessing LittleFS storage, and cross-linking with the interactive fallback recovery interface.

#### Scenario: Navigating to /device when LittleFS is unformatted or corrupted
- **WHEN** a client navigates to `http://10.10.10.10/device` or `http://10.10.10.10/device.html` while `SYSTEM_FS` is completely unformatted or corrupt
- **THEN** the server returns HTTP 200 OK and serves the complete diagnostics page with intact styling and scripts.

#### Scenario: Navigating from recovery fallback to diagnostics
- **WHEN** user clicks the diagnostics button on the uninitialized partition recovery page
- **THEN** the browser opens `/device` with full hands-free developer tools available.

### Requirement: Detailed Board & Hardware Capability Reporting
The diagnostics page SHALL query `/api/board/manifest` and `/api/v1/system/telemetry` to display MCU details (chip model, cores, clock speed, flash/SRAM geometry), power metrics (V_SYS, V_TARGET, current sense, core temperature), and the complete 30-pin Dynamic Pin Arbiter ownership matrix.

#### Scenario: Displaying live hardware metrics
- **WHEN** the diagnostics page loads in a client browser
- **THEN** it renders MCU specifications, supply voltages, current consumption, temperature, and a visual matrix of GPIO 0 to 29 showing whether each pin is `free` or claimed by an active subsystem.

### Requirement: Visual Flash Partition Map & Dedicated Partition Cards
The diagnostics page SHALL display a proportional segmented linear memory bar and geometry table representing the Flash layout (`BOOT_TABLE`, `FW_SLOT_A`, `FW_SLOT_B`, `SYSTEM_FS`, `USER_FS`, `TRAIL`), with dedicated cards for `WebUI & Storage (SYSTEM_FS)` and `User Storage (USER_FS)` containing in-place formatting controls.

#### Scenario: Viewing memory geometry and partition state
- **WHEN** the diagnostics page loads
- **THEN** it highlights the currently active firmware execution slot in the segmented linear bar and displays offset boundaries and LittleFS headroom for both `SYSTEM_FS` (512 KB at `0x00260000`) and `USER_FS` (1136 KB at `0x002E0000`).

### Requirement: In-Page Action Confirmation Modal
Destructive developer actions (reboot, BOOTSEL entry, slot rollback, pin release, partition format) SHALL prompt the user via an in-page Neo-Industrial confirmation modal dialog rather than standard browser dialogs (`confirm()` / `alert()`).

#### Scenario: Triggering a destructive partition format or reboot
- **WHEN** the user clicks a destructive action button
- **THEN** an in-page modal dialog appears with an action description and high-contrast `[ CANCEL ]` and `[ EXECUTE ACTION ]` buttons, awaiting explicit user confirmation before issuing the REST call.

### Requirement: In-Browser Update Checker with Individual Flashers
The diagnostics page SHALL provide an in-browser update checker that queries the GitHub Releases API using the client's internet connection, compares installed versions against remote releases, and provides distinct, separate update actions for Firmware and WebUI.

#### Scenario: Detecting update availability
- **WHEN** the user opens the diagnostics page while connected to the internet
- **THEN** client JavaScript compares the installed firmware version (from `/api/v1/system/firmware/status`) and WebUI asset manifest against the latest GitHub release and displays version tags and update status badges.

#### Scenario: Individual firmware update
- **WHEN** the user initiates a firmware update via 1-click download or file picker
- **THEN** the client streams the firmware binary to `POST /api/v1/system/firmware/ota`, displays upload progress, and prompts to reboot into the candidate slot.

#### Scenario: Individual WebUI update
- **WHEN** the user initiates a WebUI update via 1-click download or file picker
- **THEN** the client streams the update tarball to `POST /api/v1/system/webui/update`, displays extraction progress, and updates `SYSTEM_FS` without requiring a firmware reboot.
