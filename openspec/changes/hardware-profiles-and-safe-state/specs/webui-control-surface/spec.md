## MODIFIED Requirements

### Requirement: WebUI lifecycle initialization & Start menu control
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, query `/api/profiles` to determine the active and saved hardware profiles, and render the top-level profile switcher and emergency safe state button alongside primary workbench modes.

#### Scenario: Successful WebUI boot with profiles
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** the top navigation bar SHALL display `[ 🛡️ SAFE STATE ]`, the active profile badge (e.g. `[ Profile: Safe State ▾ ]`), and populate the profile dropdown list

#### Scenario: Switching primary modes
- **WHEN** the user clicks `#nav-target-station` or `#nav-active-bench`
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

## ADDED Requirements

### Requirement: Cross-Board Graceful Partial Loading
When loading a profile configured on a different hardware profile, the WebUI SHALL apply all compatible pins and instruments and display an informative modal listing skipped or unsupported features.

#### Scenario: Loading BakuProbe profile on Pico 2
- **WHEN** a profile originating from `bakuprobe` containing CAN-FD, RS-485, or GP30-GP47 is loaded on a `pico2` device
- **THEN** the WebUI SHALL apply all valid GP0-GP29 assignments, omit CAN-FD/RS-485, and display a compatibility alert summarizing applied vs skipped resources

### Requirement: Profile Management Interface
The WebUI SHALL provide a modal dialog allowing users to save the current workbench configuration to an available flash slot, rename profiles, designate a boot default, or delete slots.

#### Scenario: Saving new profile
- **WHEN** the user clicks "Save as Profile...", enters a name, and submits
- **THEN** the WebUI SHALL send the serialized state to `POST /api/profiles` and update the active profile indicator upon success
