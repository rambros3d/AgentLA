# adaptive-workspace-hub Specification

## MODIFIED Requirements

### Requirement: Touch and Desktop Adaptive Navigation Shell
The WebUI SHALL provide an adaptive viewport shell that presents navigation controls for the unified workspaces, with the primary observation instrument mode named **"Scope"** across desktop navigation (in-flow 48px left vertical sidebar below the top bar), mobile navigation bar (in-flow 48px full-width bottom bar), workspace tab headers, and keyboard shortcut indicators (`Scope (Hotkey: 1)`). Navigation destinations SHALL render Solar Duotone SVG icons (`graph` for Scope, `link-circle` for Adapters, `magnifier-bug` for Debugger, `cpu`/`pinout` for Pin Matrix, and `programming` for Console). On wide viewports (`>= 900px`), navigation items SHALL stack vertically in a 48px sidebar on the left margin, reserving 100% vertical display height for the active workspace; on compact viewports (`< 900px`), navigation items SHALL arrange horizontally across the bottom edge. Workspace view containers SHALL NOT inject artificial bottom padding or dead margins to clear floating overlays.

#### Scenario: Scope mode navigation label on desktop
- **WHEN** viewing the desktop left vertical sidebar navigation
- **THEN** the primary observation mode tab renders the Solar `graph` icon within a 48px touch target and navigates to the unified Scope workspace view without overlapping content

#### Scenario: Scope mode navigation label on mobile
- **WHEN** viewing the mobile bottom navigation bar
- **THEN** the primary observation tab displays the text or Solar `graph` icon within the bottom bar and switches to the Scope view

#### Scenario: Clean workspace layout without dead margins
- **WHEN** viewing any active workspace view (Scope, Adapters, Debugger, Pins, or Console)
- **THEN** the workspace container expands to full flex boundaries with zero artificial bottom clearance padding
