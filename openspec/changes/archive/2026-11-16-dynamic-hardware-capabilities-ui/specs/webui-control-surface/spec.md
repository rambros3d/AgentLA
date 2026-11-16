## MODIFIED Requirements

### Requirement: WebUI lifecycle initialization & Start menu control
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, load `/board.json`, and dynamically initialize the top-level navigation, board branding, mode controls, and hardware capability adaptations based on the probe's declared hardware profile.

#### Scenario: Successful WebUI boot
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** `#loading-screen` receives the `fade-out` class within 500ms, the board branding header displays the board's name and `logo.svg`, the top bar displays `[ Active Bench ]` as the default active mode, and `FeatureRegistry` reconciles all capability-dependent DOM controls against `board.json`

#### Scenario: Switching primary modes
- **WHEN** the user clicks `#nav-target-station` or `#nav-active-bench`
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

## ADDED Requirements

### Requirement: Accurate Level Translation and Zone Tagging
The dynamic pin matrix and domain cards SHALL derive level translation attributes strictly from `manifest.level_shifting` and zone definitions rather than displaying unverified level-shifting claims.

#### Scenario: Direct voltage board without level shifters
- **WHEN** `board.json` contains no `level_shifting` configuration for programmer pins (e.g. Raspberry Pi Pico 2)
- **THEN** the programmer zone domain tag SHALL display `3.3V Direct SIO` and SHALL NOT display `1.2V–5.0V Level-Shifted`
