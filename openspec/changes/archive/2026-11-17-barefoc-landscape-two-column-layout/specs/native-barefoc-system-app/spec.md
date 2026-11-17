## ADDED Requirements

### Requirement: Responsive Two-Column Landscape Layout
The system SHALL present the BareFOC Studio dashboard (`#view-dashboard`) in a responsive two-column grid on wide screens (`lg:` breakpoint and above), arranging live telemetry and phase vector diagnostics in the left column and motion control modes and the radial setpoint dial in the right column to fit within a single landscape viewport without vertical scrolling.

#### Scenario: Viewing dashboard on wide landscape display
- **WHEN** user opens BareFOC Studio on a wide display (viewport width >= 1024px)
- **THEN** the dashboard renders with live telemetry and phase vector diagnostics in the left 5 columns and motion modes and the interactive radial dial in the right 7 columns, fitting the viewport without vertical scrolling

#### Scenario: Viewing dashboard on mobile or narrow display
- **WHEN** user opens BareFOC Studio on a narrow display (viewport width < 1024px)
- **THEN** the dashboard collapses into a single vertical column preserving full functional accessibility
