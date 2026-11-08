# dockable-canvas-telemetry Specification

## Purpose
Provides dockable live canvas widgets on the WebUI home workspace for real-time target telemetry (voltage, current, power, core temperature, GPIO crossbar) and flash storage status, interoperating with floating rail dock popovers and persisting widget states in `localStorage`.

## Requirements

### Requirement: Workspace Telemetry HUD Widget
The WebUI App Launcher and home workspace SHALL provide an embedded or dockable live Telemetry HUD widget displaying real-time target voltage (`V_TARGET`), target current (`I_TARGET`), power consumption (`POWER`), MCU core temperature (`CORE TEMP`), and the 30-pin GPIO claims crossbar matrix (`GP0..GP29`).

#### Scenario: Rendering pinned telemetry widget
- **WHEN** the user pins the Telemetry HUD to the workspace
- **THEN** a sharp 0.5rem squircle card renders at the top of the workspace canvas with live-updating metrics from `/api/v1/system/telemetry` and discrete square GPIO status cells.

#### Scenario: Unpinning telemetry widget
- **WHEN** the user clicks "Unpin" or "Close" on the pinned workspace widget
- **THEN** the widget is removed from the canvas, and telemetry access remains available via the floating dock popover.

### Requirement: Workspace Flash Storage Widget
The WebUI workspace SHALL support pinning the Flash Partitioning and System Specifications widget onto the canvas.

#### Scenario: Rendering pinned flash storage widget
- **WHEN** the user pins the Flash Storage widget to the workspace
- **THEN** the widget renders with processor clock/SRAM readouts and a segmented LED progress gauge showing ROM, User Apps, and Free LittleFS partition space.

### Requirement: Dual Dock Popover and Pinning State Persistence
The floating dock SHALL continue to host quick-launch popovers for Telemetry and Storage, with each popover including a "Pin to Workspace" action, and all widget mount states SHALL persist in `localStorage`.

#### Scenario: Toggling telemetry popover from dock
- **WHEN** user clicks the telemetry button in the floating dock
- **THEN** the floating popover toggles open with an action button to "Pin to Workspace".

#### Scenario: Persisting pinned widget state
- **WHEN** widgets are pinned or unpinned on the canvas
- **THEN** the active widget list is saved to `localStorage` under `agentla_pinned_widgets` and restored without visual flicker on subsequent page loads.
