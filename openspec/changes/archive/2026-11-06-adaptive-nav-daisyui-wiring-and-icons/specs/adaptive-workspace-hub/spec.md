## MODIFIED Requirements

### Requirement: Touch and Desktop Adaptive Navigation Shell
The WebUI SHALL provide an adaptive viewport shell that presents navigation controls for the unified workspaces, with the primary observation instrument mode named **"Scope"** across desktop navigation (vertical sidebar with -90° rotated word label `SCOPE`), mobile navigation dock, workspace tab headers, and keyboard shortcut indicators (`Scope (Hotkey: 1)`). Navigation destinations SHALL render Solar Duotone SVG icons (`graph` for Scope, `link-circle` for Adapters, `magnifier-bug` for Debugger, `cpu`/`pinout` for Pin Matrix, and `programming` for Console). On compact viewports (`< 580px`), the bottom navigation bar SHALL dynamically scale tabs such that inactive tabs collapse to snug icon-only touch targets and the active tab expands to an illuminated pill containing both icon and label; on roomy viewports (`580px` to `899px`), all tabs SHALL simultaneously display both icon and label inline.

#### Scenario: Scope mode navigation label on desktop
- **WHEN** viewing the desktop left vertical sidebar navigation
- **THEN** the primary observation mode tab renders the label `SCOPE` in uppercase rotated -90° with the Solar `graph` icon and navigates to the unified Scope workspace view

#### Scenario: Scope mode navigation label on mobile
- **WHEN** viewing the mobile bottom navigation dock
- **THEN** the primary observation tab displays the text `Scope` with the Solar `graph` icon and switches to the Scope view

#### Scenario: Compact navigation dynamic content-aware pill scaling
- **WHEN** the viewport width is below 580px
- **THEN** inactive navigation tabs display only their icon within a compact touch target
- **AND** the active navigation tab smoothly expands to display both its icon and text label within a styled pill

#### Scenario: Roomy navigation simultaneous icon and label display
- **WHEN** the viewport width is between 580px and 899px
- **THEN** all navigation tabs simultaneously display both their Solar Duotone icon and text label inline without truncating or hiding inactive labels
