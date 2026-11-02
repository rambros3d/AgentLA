## MODIFIED Requirements

### Requirement: Touch and Desktop Adaptive Navigation Shell
The WebUI SHALL provide an adaptive viewport shell that presents navigation controls for the unified workspaces, with the primary observation instrument mode named **"Scope"** across desktop navigation (vertical sidebar with -90° rotated word label `SCOPE`), mobile navigation dock, workspace tab headers, and keyboard shortcut indicators (`Scope (Hotkey: 1)`).

#### Scenario: Scope mode navigation label on desktop
- **WHEN** viewing the desktop left vertical sidebar navigation
- **THEN** the primary observation mode tab renders the label `SCOPE` in uppercase rotated -90° and navigates to the unified Scope workspace view

#### Scenario: Scope mode navigation label on mobile
- **WHEN** viewing the mobile bottom navigation dock
- **THEN** the primary observation tab displays the text `Scope` with the waveform icon and switches to the Scope view
