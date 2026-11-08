## ADDED Requirements

### Requirement: Default High-Z Boot State
The firmware SHALL initialize all flexible and zone GPIO pins as High-Z inputs with pulls disabled and target power rail (VTARGET) at 0.0V on power-on and reset before any profile is applied.

#### Scenario: Normal power-on
- **WHEN** the probe powers up or reboots
- **THEN** all probe pins SHALL remain in High-Z input mode with target power off until a profile is loaded or the user claims a resource

### Requirement: Immediate Emergency Safe State Abort
The system SHALL provide an immediate emergency safe state mechanism accessible via the WebUI and keyboard hotkey that instantly disables target power and tri-states all pins.

#### Scenario: Emergency trigger via WebUI button
- **WHEN** the user clicks the `[ SAFE STATE ]` button in the WebUI header
- **THEN** the probe SHALL cut VTARGET within 1ms, set all GPIOs to High-Z inputs, abort all active captures, and return the workbench to Safe State

#### Scenario: Emergency trigger via keyboard hotkey
- **WHEN** the user presses `Shift+Esc` (or `Esc` outside input elements) in the WebUI
- **THEN** the WebUI SHALL immediately dispatch the safe state command to `/api/profiles/safe_state` and display an emergency safety alert
