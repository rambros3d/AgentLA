# adaptive-navigation-rail Specification

## Purpose
Establishes an in-flow, responsive navigation rail that presents primary application links and system telemetry triggers via an icon-only 48px vertical sidebar on wide viewports and a 48px full-width horizontal bottom bar on narrow viewports without floating over workspace content.

## ADDED Requirements

### Requirement: Desktop Left Vertical Navigation Rail
The WebUI SHALL display an in-flow vertical navigation rail anchored to the left viewport edge below the 100% full-width top navigation bar on viewports with width >= 900px, occupying a fixed 48px width (`border-r border-base-300 bg-base-200`). The workspace SHALL flex dynamically into the remaining horizontal space without overlapping.

#### Scenario: Rendering navigation rail on desktop
- **WHEN** user loads any WebUI page on a viewport >= 900px wide
- **THEN** the navigation rail displays vertically on the left edge directly beneath the top bar
- **AND** the main workspace occupies `width: calc(100vw - 48px)` with zero content overlap

#### Scenario: Desktop navigation item interaction
- **WHEN** user hovers over an icon button in the left navigation rail
- **THEN** a tooltip flyout displays to the right of the button
- **AND** clicking switches the active workspace view while applying active surface styling without dot indicators

### Requirement: Mobile Full-Width Bottom Navigation Bar
The WebUI SHALL display an in-flow horizontal navigation bar anchored directly flush with the bottom viewport edge on viewports with width < 900px, occupying a fixed 48px height (`border-t border-base-300 bg-base-200`) and respecting safe-area-inset-bottom.

#### Scenario: Rendering bottom bar on mobile
- **WHEN** user loads any WebUI page on a viewport < 900px wide
- **THEN** the navigation rail displays horizontally across the full width of the bottom edge
- **AND** the main workspace occupies `height: calc(100vh - 44px - 48px)` with zero content overlap

### Requirement: Adjacent Popover Docking
The Hardware Telemetry HUD and Device Storage popover panels SHALL anchor directly adjacent to their respective navigation trigger buttons without floating unattached.

#### Scenario: Opening telemetry HUD on desktop
- **WHEN** user clicks the Telemetry button on a viewport >= 900px wide
- **THEN** the telemetry HUD popover opens anchored to the right of the vertical rail (`left: calc(100% + 8px)`)

#### Scenario: Opening telemetry HUD on mobile
- **WHEN** user clicks the Telemetry button on a viewport < 900px wide
- **THEN** the telemetry HUD popover opens anchored above the bottom navigation bar (`bottom: calc(100% + 8px)`)
