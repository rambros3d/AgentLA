# app-installer-modal Specification

## Purpose
Provides an interactive installation preview modal displaying app metadata, storage budget impact, and installation progress.

## Requirements

### Requirement: Client-Side TAR Metadata Extraction
Prior to uploading an app package to the device, the App Launcher frontend SHALL unpack the archive headers in memory to extract `app.json` and inlined icon assets.

#### Scenario: Inspecting dropped TAR package
- **WHEN** user drops or selects an app `.tar` file in the launcher
- **THEN** the client-side parser extracts `app.json`, decodes the app icon, and presents the installation confirmation modal without uploading the archive yet.

### Requirement: Flash Storage Capacity Verification
The installer modal SHALL query device flash capacity via `/api/v1/apps/storage` and verify that sufficient free flash storage exists to accommodate the unpacked application footprint.

#### Scenario: Flash space sufficient
- **WHEN** the estimated extracted footprint of the app is less than or equal to `free_bytes`
- **THEN** the modal displays flash storage status as sufficient (e.g. "380 KB needed, 14.2 MB available") and keeps the "Confirm & Install" button enabled.

#### Scenario: Flash space insufficient
- **WHEN** the estimated extracted footprint of the app exceeds `free_bytes`
- **THEN** the modal displays an error banner ("Insufficient Flash Space"), shows exact bytes needed versus available, and disables the "Confirm & Install" button.

### Requirement: Hardware Compatibility Verification
The installer modal SHALL verify any hardware peripherals or firmware versions declared in `app.json` against the active board manifest.

#### Scenario: Compatible hardware configuration
- **WHEN** all required peripherals declared in `app.json` are supported by the connected board
- **THEN** the modal displays a green compatibility badge ("Compatible with your board").

#### Scenario: Missing optional or board-specific peripheral
- **WHEN** an app declares a requirement not present on the active board (such as an on-board CAN transceiver on a standard Pico 2)
- **THEN** the modal displays an advisory warning banner with an override checkbox ("Install anyway") before allowing confirmation.

### Requirement: Overwrite and Upgrade Confirmation
The installer modal SHALL identify whether an app with the same name or identifier is already installed on flash.

#### Scenario: Upgrading an existing app
- **WHEN** an app with matching identifier already exists on device
- **THEN** the modal displays an upgrade notice showing current version and new version, and upon confirmation replaces existing files cleanly.
