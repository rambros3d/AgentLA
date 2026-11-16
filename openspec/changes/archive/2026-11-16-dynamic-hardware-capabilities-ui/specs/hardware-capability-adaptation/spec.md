## ADDED Requirements

### Requirement: Dynamic UI Capability Gating
The WebUI SHALL evaluate the loaded board manifest (`board.json`) upon initialization and conditionally render, style, or disable user interface elements corresponding to hardware peripherals.

#### Scenario: Board without target power control
- **WHEN** the active board manifest specifies `features.target_power_control: false`
- **THEN** the WebUI SHALL disable or hide the target power toggle switch (`#target-power-switch`) and prevent click events on `#btn-target-power-pill` from dispatching power state change commands

#### Scenario: Board without target current sense
- **WHEN** the active board manifest specifies `features.target_current_sense: false`
- **THEN** the WebUI SHALL hide target current (mA) and target power (mW) metrics from `#top-bar-power-text` and `#target-power-telemetry`

### Requirement: App Store Hardware Compatibility Verification
The App Store catalog and installation interfaces SHALL cross-reference an application's declared `requires` array against the active `BOARD_MANIFEST.features`.

#### Scenario: Application requires unsupported hardware
- **WHEN** user views an application whose `requires` includes a capability that is `false` or missing in `BOARD_MANIFEST.features` (such as `has_hardware_can`)
- **THEN** the interface SHALL render a visible hardware incompatibility warning badge and inform the user of the missing onboard peripheral

#### Scenario: Application requirements satisfied
- **WHEN** user views an application whose required capabilities are all `true` in `BOARD_MANIFEST.features`
- **THEN** the interface SHALL display a satisfied compatibility badge confirming hardware readiness
