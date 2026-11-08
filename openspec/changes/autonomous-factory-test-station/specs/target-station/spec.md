## MODIFIED Requirements

### Requirement: Consolidated Target Station Surface
The WebUI SHALL provide a dedicated Target Station view accessible from the top-level navigation that groups target programming, verification, live debugging, and autonomous factory recipe execution into a unified workspace.

#### Scenario: Switching to Target Station
- **WHEN** the user selects the `[ Target Station ]` navigation tab
- **THEN** the active bench view SHALL be hidden and the Target Station view SHALL become active

#### Scenario: Target Power Control
- **WHEN** the user toggles the VTarget power rail switch in Target Station
- **THEN** the system SHALL send the power command to the probe and visually indicate the rail status and measured voltage

#### Scenario: Activating Autonomous Factory Mode
- **WHEN** the user toggles the "Autonomous Production Mode" switch in Target Station
- **THEN** the interface SHALL transition to the dedicated full-screen Production Station surface and await contact trigger events
