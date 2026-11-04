# unified-os-app-launcher Specification

## Purpose
TBD - created by archiving change lab-route-and-home-redirect. Update Purpose after archive.
## Requirements
### Requirement: OS-Style Squircle App Grid
The App Launcher (`/apps`) SHALL render applications as a responsive grid of tactile squircle tiles using flat DaisyUI `.card` surfaces with 1px borders (`border-base-300`), featuring an app icon, application name, category or version badge, and spring hover animations without neumorphic drop shadows or white rim bevels.

#### Scenario: App tile rendering
- **WHEN** the user views the installed apps section on `/apps`
- **THEN** each app appears as a squircle tile with a 56×56px icon styled with flat `base-200` background and 1px `base-300` border, and clicking/tapping anywhere on the tile launches the app directly.

#### Scenario: Flagship AgentLA Lab tile
- **WHEN** the launcher loads
- **THEN** "AgentLA Lab" is displayed as the primary first tile in the grid with a `FLAGSHIP` badge and clicking it navigates to `/lab`.

### Requirement: Spotlight Search Filter
The App Launcher SHALL provide a centered spotlight search bar that performs live fuzzy filtering on the app grid by name, tags, and description.

#### Scenario: User types in search bar
- **WHEN** the user enters a search term in the spotlight search bar
- **THEN** only matching app tiles remain visible in the grid in real time.

### Requirement: Segmented Storage Capacity Indicator
The App Launcher SHALL render a segmented storage capacity bar displaying system firmware ROM, installed LittleFS applications, and free available flash space.

#### Scenario: Storage telemetry update
- **WHEN** `/api/v1/storage` returns partition metrics
- **THEN** the storage bar visually segments used and available bytes with formatted byte labels.

### Requirement: Unified Top Bar and Shared Settings Modal
The App Launcher SHALL share the exact same top bar layout and `#modal-settings` System Settings modal as the workbench, implemented using a standard DaisyUI `.navbar` (`bg-base-100`, `border-b border-base-300`) without frosted backdrop blur, omitting redundant "Workbench" and "App Launcher" navigation pills.

#### Scenario: Opening settings from launcher
- **WHEN** the user clicks the gear icon (`⚙`) in the launcher top bar
- **THEN** the `#modal-settings` dialog opens, providing access to Theme Mode, Default Landing Page, Device Reboot, BOOTSEL mode, and Hardware Telemetry.

#### Scenario: Switching theme from launcher
- **WHEN** the user toggles the theme button in the launcher top bar
- **THEN** the active theme cycles through the configured DaisyUI palettes and persists to `localStorage`.

