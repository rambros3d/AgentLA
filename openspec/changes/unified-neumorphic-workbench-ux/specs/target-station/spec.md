## MODIFIED Requirements

### Requirement: Consolidated Target Station Surface
The WebUI SHALL provide a dedicated Target Station / Debugger view (`#view-target-station`) organized in an uncluttered two-column layout consisting of the Universal Target Flasher on the left and the Target SWD / CMSIS-DAP Debugger on the right, without hosting a dedicated power rail card.

#### Scenario: Switching to Target Station
- **WHEN** the user selects the Debugger navigation tab
- **THEN** the active bench view SHALL be hidden and the Target Station view SHALL become active displaying the Flasher column and Debugger column side-by-side
