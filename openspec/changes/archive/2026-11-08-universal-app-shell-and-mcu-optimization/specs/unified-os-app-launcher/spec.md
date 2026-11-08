## MODIFIED Requirements

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
