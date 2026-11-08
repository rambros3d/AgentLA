## ADDED Requirements

### Requirement: File Manager Launch Tile
The App Launcher (`/` or `/apps`) SHALL render a core instrument squircle tile for "Files" (Bento Storage Explorer) in the System Instruments grid with Solar Icon graphics, linking to `files.html` or `/files`, and supporting bottom dock pinning.

#### Scenario: Clicking Files tile from launcher
- **WHEN** user clicks the "Files" tile in the launcher grid
- **THEN** the browser navigates to `/files` (or `files.html` in demo mode).

#### Scenario: Pinning Files to dock
- **WHEN** user pins the Files application to the OS bottom dock
- **THEN** a quick-launch squircle icon appears in the bottom dock and persists across browser sessions.
