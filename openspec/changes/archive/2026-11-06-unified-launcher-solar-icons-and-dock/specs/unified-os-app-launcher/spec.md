## MODIFIED Requirements

### Requirement: OS-Style Squircle App Grid
The App Launcher (`/apps`) SHALL render core instruments and installed applications as a responsive grid of tactile squircle tiles using flat DaisyUI `.card` surfaces with 1px borders (`border-base-300`), featuring authentic 2-tone Solar Icons for core apps, support for custom user-app icons, and spring hover animations without neumorphic drop shadows or white rim bevels. The core section SHALL be titled "Core Apps".

#### Scenario: Core apps rendering with Solar icons
- **WHEN** the user loads `/apps`
- **THEN** the "Core Apps" section displays tiles for "AgentLA Lab", "MSO Scope", "Pinout Matrix", "USB Adapters", and "Console" using authentic 2-tone Solar Icons (`solar-icon`) matching the Lab navigation rail, and the "AgentLA Lab" tile features a `FLAGSHIP` badge.

#### Scenario: User installed app custom icon rendering
- **WHEN** a user-installed LittleFS app provides an `iconSvg` string in its `app.json` or an `icon.svg`/`icon.png` in its app root
- **THEN** the app launcher renders the custom graphic inside the 56×56px squircle icon container.

#### Scenario: User installed app fallback Solar icon
- **WHEN** a user-installed LittleFS app provides neither an `iconSvg` nor an icon image file
- **THEN** the app launcher renders an authentic 2-tone Solar application box icon as the tile graphic.

### Requirement: Unified Top Bar and Shared Settings Modal
The App Launcher SHALL share the exact same top bar layout and `#modal-settings` System Settings modal as the workbench, implemented using a standard DaisyUI `.navbar` (`bg-base-100`, `border-b border-base-300`), including the Hardware Profile selector dropdown (`#profile-dropdown-wrapper`) wired to `/api/profiles`, while keeping the brand logo (`#brand-logo`) display-only on the launcher.

#### Scenario: Switching hardware profile from launcher top bar
- **WHEN** the user clicks `#btn-profile-toggle` in the launcher top bar and selects a profile
- **THEN** the system issues a POST request to `/api/profiles/active` and updates the active profile badge without requiring entry into the Lab workbench.

#### Scenario: Top bar brand logo behavior
- **WHEN** the user views the launcher top bar
- **THEN** the `#brand-logo` renders with the online status indicator as a non-navigating branding item, and launching the Lab is performed via the "Core Apps" flagship tile.

### Requirement: Segmented Storage Capacity Indicator
The App Launcher SHALL render system firmware ROM, installed LittleFS applications, and free available flash space within a fixed DaisyUI OS bottom dock (`.dock`) anchored to the bottom viewport.

#### Scenario: OS bottom dock storage indicator rendering
- **WHEN** `/api/v1/apps/storage` returns partition metrics
- **THEN** the fixed bottom dock displays a storage icon, a segmented progress bar (ROM: `var(--color-info)`, Apps: `var(--color-primary)`, Free: `var(--color-base-300)`), and formatted capacity metrics, while the main workspace applies bottom padding to prevent content obstruction.
