# unified-os-app-launcher Specification

## Purpose
Provides an OS-style squircle application launcher grid, spotlight search, fixed bottom dock storage monitor, and unified navigation bar.
## Requirements
### Requirement: OS-Style Squircle App Grid
The App Launcher (`/apps` or `index.html`) SHALL render core instruments and installed applications as a responsive grid of tactile squircle tiles using flat DaisyUI `.card` surfaces with 1px borders (`border-base-300`), featuring authentic 2-tone Solar Icons for core apps, support for custom user-app icons, and spring hover animations without neumorphic drop shadows or white rim bevels. The core section SHALL be titled "Core Apps". Clicking any core instrument or installed app tile SHALL launch the target application within the persistent App-Shell viewport (`#view-app-viewport`) via client-side hash navigation rather than performing a full-page browser reload.

#### Scenario: App tile rendering
- **WHEN** the user views the installed apps section on `/apps`
- **THEN** each app appears as a squircle tile with a 56×56px icon styled with flat `base-200` background and 1px `base-300` border, and clicking/tapping anywhere on the tile launches the app inside the App-Shell viewport via URL hash navigation.

#### Scenario: Flagship AgentLA Lab tile
- **WHEN** the launcher loads
- **THEN** "AgentLA Lab" is displayed as the primary first tile in the grid with a `FLAGSHIP` badge and clicking it navigates to the Lab view inside the App-Shell.

#### Scenario: Core apps rendering with Solar icons
- **WHEN** the user loads `/apps`
- **THEN** the "Core Apps" section displays tiles for "AgentLA Lab", "MSO Scope", "Pinout Matrix", "USB Adapters", and "Console" using authentic 2-tone Solar Icons (`solar-icon`) matching the Lab navigation rail, and the "AgentLA Lab" tile features a `FLAGSHIP` badge.

#### Scenario: User installed app custom icon rendering
- **WHEN** a user-installed LittleFS app provides an `iconSvg` string in its `app.json` or an `icon.svg`/`icon.png` in its app root
- **THEN** the app launcher renders the custom graphic inside the 56×56px squircle icon container.

#### Scenario: User installed app fallback Solar icon
- **WHEN** a user-installed LittleFS app provides neither an `iconSvg` nor an icon image file
- **THEN** the app launcher renders an authentic 2-tone Solar application box icon as the tile graphic.

### Requirement: Spotlight Search Filter
The App Launcher SHALL provide a centered spotlight search bar that performs live fuzzy filtering on the app grid by name, tags, and description.

#### Scenario: User types in search bar
- **WHEN** the user enters a search term in the spotlight search bar
- **THEN** only matching app tiles remain visible in the grid in real time.

### Requirement: Segmented Storage Capacity Indicator
The App Launcher SHALL render system firmware ROM, installed LittleFS applications, and free available flash space within a fixed DaisyUI OS bottom dock (`.dock`) anchored to the bottom viewport.

#### Scenario: Storage telemetry update
- **WHEN** `/api/v1/storage` returns partition metrics
- **THEN** the storage bar visually segments used and available bytes with formatted byte labels.

#### Scenario: OS bottom dock storage indicator rendering
- **WHEN** `/api/v1/apps/storage` returns partition metrics
- **THEN** the fixed bottom dock displays a storage icon, a segmented progress bar (ROM: `var(--color-info)`, Apps: `var(--color-primary)`, Free: `var(--color-base-300)`), and formatted capacity metrics, while the main workspace applies bottom padding to prevent content obstruction.

### Requirement: Unified Top Bar and Shared Settings Modal
The App Launcher SHALL share the exact same top bar layout and `#modal-settings` System Settings modal as the workbench, implemented using a standard DaisyUI `.navbar` (`bg-base-100`, `border-b border-base-300`), including the Hardware Profile selector dropdown (`#profile-dropdown-wrapper`) wired to `/api/profiles`, while keeping the brand logo (`#brand-logo`) display-only on the launcher.

#### Scenario: Opening settings from launcher
- **WHEN** the user clicks the gear icon (``) in the launcher top bar
- **THEN** the `#modal-settings` dialog opens, providing access to Theme Mode, Default Landing Page, Device Reboot, BOOTSEL mode, and Hardware Telemetry.

#### Scenario: Switching theme from launcher
- **WHEN** the user toggles the theme button in the launcher top bar
- **THEN** the active theme cycles through the configured DaisyUI palettes and persists to `localStorage`.

#### Scenario: Switching hardware profile from launcher top bar
- **WHEN** the user clicks `#btn-profile-toggle` in the launcher top bar and selects a profile
- **THEN** the system issues a POST request to `/api/profiles/active` and updates the active profile badge without requiring entry into the Lab workbench.

#### Scenario: Top bar brand logo behavior
- **WHEN** the user views the launcher top bar
- **THEN** the `#brand-logo` renders with the online status indicator as a non-navigating branding item, and launching the Lab is performed via the "Core Apps" flagship tile.

### Requirement: Industrial Presentation for App Launcher Tiles
Under `data-style="industrial"`, the App Launcher SHALL render system instrument and installed application tiles as crisp technical boxes with dot-matrix labels, monospaced category tags, and Nothing Signal Red accent dots.

#### Scenario: Launcher grid in industrial mode
- **WHEN** user views `/` under `data-style="industrial"`
- **THEN** app tiles render with 0.5rem corner radiuses, 1px subtle outlines, dot-matrix titles (`font-family: 'NDot57'`), and signal red indicator lights.

#### Scenario: Launcher grid in bento mode
- **WHEN** user switches to `data-style="bento"`
- **THEN** app tiles restore their consumer rounded squircle geometry (`1.25rem` radius) and soft sans-serif typography.

### Requirement: File Manager Launch Tile
The App Launcher (`/` or `/apps`) SHALL render a core instrument squircle tile for "Files" (Bento Storage Explorer) in the System Instruments grid with Solar Icon graphics, linking to `files.html` or `/files`, and supporting bottom dock pinning.

#### Scenario: Clicking Files tile from launcher
- **WHEN** user clicks the "Files" tile in the launcher grid
- **THEN** the browser navigates to `/files` (or `files.html` in demo mode).

#### Scenario: Pinning Files to dock
- **WHEN** user pins the Files application to the OS bottom dock
- **THEN** a quick-launch squircle icon appears in the bottom dock and persists across browser sessions.

