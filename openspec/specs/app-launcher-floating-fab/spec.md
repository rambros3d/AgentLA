# app-launcher-floating-fab Specification

## Purpose
TBD - created by archiving change lab-route-and-home-redirect. Update Purpose after archive.
## Requirements
### Requirement: Floating Action Button Bridge Script
The system SHALL provide a client-side bridge script `/agentla-bridge.js` compiled into ROM fsdata that injects a circular glassmorphic Floating Action Button (FAB) into the host page.

#### Scenario: Script execution in installed app
- **WHEN** `/agentla-bridge.js` executes in a browser viewport
- **THEN** it attaches a closed Shadow DOM container containing a 44×44px circular glassmorphic FAB fixed at `bottom: 16px; right: 16px; z-index: 99999`.

#### Scenario: WinRAR Bold Duotone Icon Rendering
- **WHEN** the floating FAB is rendered
- **THEN** it displays the `win-rar-bold-duotone` Solar icon with multi-level duotone opacities, without text labels or extra buttons.

### Requirement: Launcher Navigation on Click
Clicking or tapping the floating FAB SHALL navigate the browser to `/apps`.

#### Scenario: User clicks floating FAB
- **WHEN** the user clicks or taps the floating FAB
- **THEN** the browser navigates to `http://10.10.10.10/apps`.

### Requirement: Opt-Out and Suppression
The floating FAB SHALL suppress injection if the current URL contains `no_launcher=1` or if `window.AgentLANoLauncher === true`.

#### Scenario: Suppressed injection via query parameter
- **WHEN** an installed app is loaded with URL query parameter `?no_launcher=1`
- **THEN** the floating FAB is not rendered in the DOM.

