## ADDED Requirements

### Requirement: Top-Left Brand Launcher Navigation
The WebUI workbench header SHALL render the top-left AgentLA brand logo and title as a navigational link pointing to `/apps`, without redundant "Workbench" or "App Launcher" navigation pills in the top bar.

#### Scenario: User clicks brand logo
- **WHEN** the user clicks the top-left brand logo in the workbench
- **THEN** the browser navigates to the App Launcher at `/apps`.

#### Scenario: Uncluttered top bar
- **WHEN** the workbench header renders
- **THEN** no redundant text navigation pills for "Workbench" or "App Launcher" are displayed.

## MODIFIED Requirements

### Requirement: Unified Dashboard Overview
The WebUI SHALL display a Unified Dashboard as the default landing view upon loading `/lab`, displaying probe health telemetry, an interactive 30-pin domain allocation map (`GPIO 0–29`), and 1-tap launcher tiles.

#### Scenario: Landing view display
- **WHEN** a user navigates to `http://10.10.10.10/lab`
- **THEN** the Dashboard workspace opens, showing probe CPU frequency, voltage status, USB mode, active target detection, and instrument launch tiles.

#### Scenario: Real-time pin domain allocation visualization
- **WHEN** any subsystem or adapter claims pins
- **THEN** the 30-pin allocation widget visually updates the pin state, function role (`SWD`, `I2C`, `I3C`, `UART`, `SPI`, `CAPTURE`), and owner handle.
