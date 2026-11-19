## ADDED Requirements

### Requirement: Stepped Setup Wizard
The studio SHALL provide a stepped setup wizard with ordered pages, visible page navigation showing active and completed pages, Back/Next/Finish controls, contextual tooltips, and gating of Next until the current page's required choice is made.

#### Scenario: Navigating wizard pages
- **WHEN** user completes a page's required choices and clicks Next
- **THEN** studio advances to the next page and marks the previous page as completed in the navigation

#### Scenario: Next gating
- **WHEN** required choices on the current page are incomplete
- **THEN** Next is disabled or shows a tooltip and does not advance

#### Scenario: Finishing early
- **WHEN** user clicks Finish
- **THEN** studio jumps to the Review and Apply page

### Requirement: Multi-Axis Motor And Encoder Setup
The wizard SHALL present per-axis motor and encoder selection pages for every discovered axis, with preset choices, configurable fields, and derived values.

#### Scenario: Motor presets and fields
- **WHEN** user selects a motor preset or custom motor on an axis page
- **THEN** studio records pole pairs, torque constant or KV, motor type, phase resistance, and phase inductance for that axis

#### Scenario: Hall encoder CPR derivation
- **WHEN** user selects a Hall-effect encoder for an axis
- **THEN** studio derives encoder CPR as six times the configured pole pairs

#### Scenario: Control mode and limits
- **WHEN** user selects a control mode and enters velocity/current limits for an axis
- **THEN** studio records those values in the wizard configuration for that axis

### Requirement: Hardware-Driven Calibration In Wizard
The wizard SHALL provide motor calibration and encoder calibration actions per axis that invoke real adapter calibration routines, display in-progress and success/failure status, and suppress expected calibration errors while a calibration runs.

#### Scenario: Motor calibration action
- **WHEN** user requests motor calibration
- **THEN** studio invokes the target's motor calibration routine, shows a Calibrating state, then reports Calibration succeeded or Calibration failed and stores measured phase resistance and inductance on success

#### Scenario: Encoder calibration action
- **WHEN** user requests encoder calibration
- **THEN** studio computes a suitable calibration scan distance, invokes the encoder calibration routine, and reports success or failure without relying on fixed timer delays for completion

#### Scenario: Expected error masking during calibration
- **WHEN** a calibration produces its expected phase-resistance/inductance or CPR/pole-pair errors while the wizard is active
- **THEN** the footer and wizard suppress those specific errors from being shown as unexpected failures without affecting error reporting on other pages

### Requirement: Review And Apply
The wizard SHALL include a final Review and Apply page that shows old versus new configuration values and applies all non-null changed values to the selected target.

#### Scenario: Reviewing changes
- **WHEN** user reaches the final wizard page
- **THEN** studio displays a diff of old target values versus the wizard's new values with changed values highlighted

#### Scenario: Applying configuration
- **WHEN** user clicks Apply
- **THEN** studio writes all changed configuration values to the target through its adapter and confirms completion
