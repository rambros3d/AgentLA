## ADDED Requirements

### Requirement: Industrial Presentation for App Launcher Tiles
Under `data-style="industrial"`, the App Launcher SHALL render system instrument and installed application tiles as crisp technical boxes with dot-matrix labels, monospaced category tags, and Nothing Signal Red accent dots.

#### Scenario: Launcher grid in industrial mode
- **WHEN** user views `/` under `data-style="industrial"`
- **THEN** app tiles render with 0.5rem corner radiuses, 1px subtle outlines, dot-matrix titles (`font-family: 'NDot57'`), and signal red indicator lights.

#### Scenario: Launcher grid in bento mode
- **WHEN** user switches to `data-style="bento"`
- **THEN** app tiles restore their consumer rounded squircle geometry (`1.25rem` radius) and soft sans-serif typography.
