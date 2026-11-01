## ADDED Requirements

### Requirement: Dual-Slot Dockable Workspace Management
The WebUI SHALL provide a dockable Dual-Slot split workspace that can display two instruments simultaneously without floating or overlapping windows, oriented side-by-side on wide viewports ($\ge 900\text{px}$) and stacked vertically on narrower viewports.

#### Scenario: Toggle split view mode
- **WHEN** the user toggles the split-view button in the header
- **THEN** the workspace transitions between 1-pane full focus and 2-pane docked split mode.

#### Scenario: Responsive orientation
- **WHEN** split mode is active and viewport width changes
- **THEN** the two slots automatically arrange side-by-side on wide screens ($\ge 900\text{px}$) and stack top-to-bottom on screens below 900px.

### Requirement: 1-Tap Split Presets and Custom Routing
The WebUI SHALL provide 1-tap split presets for common bench workflows as well as independent instrument selection for each slot.

#### Scenario: Bus Debug preset activation
- **WHEN** the user selects the "Bus Debug" preset
- **THEN** Slot 1 is assigned to the Logic Analyzer and Slot 2 is assigned to the Bus Adapter (I3C / I2C / SPI).

#### Scenario: Target Bring-up preset activation
- **WHEN** the user selects the "Target Bring-up" preset
- **THEN** Slot 1 is assigned to the Target Flasher and Slot 2 is assigned to the Serial Terminal.

#### Scenario: Custom slot assignment
- **WHEN** the user selects an instrument from a slot's header dropdown
- **THEN** that slot immediately mounts the selected tool while preserving the tool mounted in the adjacent slot.
