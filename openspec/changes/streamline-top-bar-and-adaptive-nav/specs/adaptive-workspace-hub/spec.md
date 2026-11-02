## MODIFIED Requirements

### Requirement: Touch and Desktop Adaptive Navigation Shell
The WebUI SHALL provide an adaptive viewport shell that automatically presents a thumb-friendly Bottom Navigation Bar on viewports below 900px, transitions to a dedicated Vertical Left Activity Sidebar on desktop viewports 900px or wider, and completely frees up the top application header for telemetry and safety utilities.

#### Scenario: Desktop vertical activity sidebar with -90 degree rotated words
- **WHEN** the WebUI is viewed on desktop screens 900px or wider
- **THEN** primary workbench mode switching renders in a vertical sidebar along the left edge of the viewport, where each tab displays its icon above its text label, with the word itself rotated by -90 degrees (`transform: rotate(-90deg)`)
- **AND** the top application header renders without mode navigation buttons

#### Scenario: Mobile and touch portrait layout with active-tab text expansion
- **WHEN** the WebUI is accessed on a viewport narrower than 900px or in touch portrait mode
- **THEN** the vertical sidebar hides and the navigation bar renders anchored at the bottom where unselected tabs display icon-only, while the active tab displays both icon and text label without overflowing the viewport width
