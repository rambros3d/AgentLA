## ADDED Requirements

### Requirement: WebUI Profile JSON-RPC and Simulation Fidelity
The WebUI client SHALL issue profile operations conforming to the JSON-RPC command bridge schema (`method=cmd&name=profile_*`), and the in-browser simulator (`mock-bridge.js`) SHALL provide simulated slot persistence and listing to enable full feature preview in demo mode.

#### Scenario: Listing hardware profiles via MCP
- **WHEN** the WebUI requests the list of hardware profiles
- **THEN** it dispatches a command with `name="profile_list"`
- **AND** the bridge returns the list of configured slots including active slot indicator and boot flag

#### Scenario: In-browser hardware profile simulation
- **WHEN** running in demo mode without physical hardware
- **THEN** `mock-bridge.js` intercepts `profile_list`, `profile_safe`, `profile_apply`, and `profile_save` calls
- **AND** returns realistic slot collections enabling interactive dropdown and modal validation
