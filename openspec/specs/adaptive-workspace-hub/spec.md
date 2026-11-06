# adaptive-workspace-hub Specification

## Purpose
Provides an adaptive navigation shell and workspace layout supporting desktop sidebar, mobile dock, touch interactions, and dashboard views.

## Requirements
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

### Requirement: Unified Dashboard Overview
The WebUI SHALL display a Unified Dashboard as the default landing view upon loading `/lab`, displaying probe health telemetry, an interactive 30-pin domain allocation map (`GPIO 0–29`), and 1-tap launcher tiles.

#### Scenario: Landing view display
- **WHEN** a user navigates to `http://10.10.10.10/lab`
- **THEN** the Dashboard workspace opens, showing probe CPU frequency, voltage status, USB mode, active target detection, and instrument launch tiles.

#### Scenario: Real-time pin domain allocation visualization
- **WHEN** any subsystem or adapter claims pins
- **THEN** the 30-pin allocation widget visually updates the pin state, function role (`SWD`, `I2C`, `I3C`, `UART`, `SPI`, `CAPTURE`), and owner handle.

### Requirement: Slide-Up Bottom Sheet and Drawer Inspection
The WebUI SHALL display tool configuration, trigger setup, and pin selection within a slide-up Bottom Sheet on mobile viewports and a slide-out Right Drawer on desktop viewports, replacing floating dialog windows.

#### Scenario: Mobile tool configuration
- **WHEN** a user taps "Configure" on a mobile viewport
- **THEN** a bottom sheet slides up from the viewport base with large form controls and closes upon downward swipe or outside tap.

#### Scenario: Desktop configuration drawer
- **WHEN** a user opens configuration on a desktop viewport
- **THEN** an inspector drawer slides out from the right margin without obscuring the primary waveform or data log.

### Requirement: Waveform Multi-Touch Navigation
The Logic Analyzer canvas SHALL support native touch gestures using `touch-action: none` for horizontal pinch-to-zoom, one/two-finger panning, and double-tap zoom reset.

#### Scenario: Touch pinch-to-zoom and pan
- **WHEN** a user pinches or drags across the waveform canvas on a touch device
- **THEN** the sample buffer zooms in/out horizontally and pans through time without triggering browser page-level zooming or scrolling.

#### Scenario: Mouse wheel zoom and drag
- **WHEN** a user uses mouse wheel or drag on desktop
- **THEN** the waveform zooms centered at the cursor and pans horizontally.

### Requirement: Top-Left Brand Launcher Navigation
The WebUI workbench header SHALL render the top-left AgentLA brand logo and title as a navigational link pointing to `/apps`, without redundant "Workbench" or "App Launcher" navigation pills in the top bar.

#### Scenario: User clicks brand logo
- **WHEN** the user clicks the top-left brand logo in the workbench
- **THEN** the browser navigates to the App Launcher at `/apps`.

#### Scenario: Uncluttered top bar
- **WHEN** the workbench header renders
- **THEN** no redundant text navigation pills for "Workbench" or "App Launcher" are displayed.

