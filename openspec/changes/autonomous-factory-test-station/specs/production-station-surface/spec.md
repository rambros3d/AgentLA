## ADDED Requirements

### Requirement: Fullscreen High-Contrast Production Station UI
The WebUI SHALL provide a dedicated, high-contrast "Production Station" view engineered for factory touch screens, displaying prominent full-screen PASS / FAIL cards readable from at least 2 meters away.

#### Scenario: Unit successfully completes test cycle
- **WHEN** all recipe steps pass successfully
- **THEN** the screen SHALL display a large green `PASS` banner, emit a pleasant audio chime, and display unit serial and cycle time in under 4 seconds

#### Scenario: Unit fails a test step
- **WHEN** any test step fails or times out
- **THEN** the screen SHALL display a large red `FAIL` banner, emit an alert buzzer tone, and prominently display the failed step name and measured vs expected values

### Requirement: Batch Production Statistics & Logging
The Production Station SHALL maintain live batch yield metrics (Total Tested, Passed, Failed, Yield Percentage, Average Cycle Time) and support export to CSV/JSON format.

#### Scenario: Live yield counter update
- **WHEN** a test completes
- **THEN** the total count and yield percentage counters SHALL update in real time without refreshing the page
