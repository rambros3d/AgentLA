## ADDED Requirements

### Requirement: Top-Bar Target Power Toggle Pill
The WebUI top application header SHALL display a live Target Power pill button showing measured voltage and current telemetry, enabling single-click power rail toggling across all workbench views.

#### Scenario: Displaying live power rail telemetry
- **WHEN** the target power rail is enabled
- **THEN** the top-bar power pill displays an active bolt icon, soft green background, and real-time voltage and current values (e.g., `3.31V · 14mA`)

#### Scenario: Toggling power rail state
- **WHEN** the user clicks the top-bar power pill while the rail is enabled
- **THEN** the WebUI dispatches `POWER OFF` over `/mcp` and updates the pill to standby state (`POWER OFF` with muted dot)
