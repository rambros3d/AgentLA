## MODIFIED Requirements

### Requirement: WebUI lifecycle initialization & Start menu control
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, load `/board.json`, and dynamically initialize the top-level navigation, board branding, and mode controls based on the probe's declared hardware profile.

#### Scenario: Successful WebUI boot
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** `#loading-screen` receives the `fade-out` class within 500ms, the board branding header displays the board's name and `logo.svg`, and the top bar displays `[ ⚡ Active Bench ]` as the default active mode

#### Scenario: Switching primary modes
- **WHEN** the user clicks `#nav-target-station` or `#nav-active-bench`
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

## ADDED Requirements

### Requirement: Capability-Driven Instrument Mounting
The WebUI SHALL dynamically show, hide, or adapt instrument navigation tabs and workspace views based on the `features` and `zones` reported by `/board.json`.

#### Scenario: Onboard differential transceivers on BakuProbe
- **WHEN** the loaded board manifest declares `has_hardware_can: true` and `has_hardware_rs485: true`
- **THEN** the Adapters tab SHALL mount the CAN-FD and RS-485 cards with onboard transceiver controls and termination toggles

#### Scenario: External Breakout Mode on boards without dedicated transceivers
- **WHEN** the loaded board manifest declares `has_hardware_can: false` or `has_hardware_rs485: false`
- **THEN** the Adapters tab SHALL present those cards in 'External Breakout Mode', displaying explicit pin connection mappings (e.g. GP18 TX, GP19 RX, GP22 Term) for wiring external transceiver breakout modules

#### Scenario: Target power and current telemetry widget
- **WHEN** the loaded board manifest declares `target_current_sense: true`
- **THEN** the WebUI header SHALL display a live INA180 mA current draw and VTARGET voltage readout
