## ADDED Requirements

### Requirement: Target And Axis Discovery
The studio SHALL discover connected motor targets and their axes across active transports and maintain their connection state in the central store.

#### Scenario: Discovering targets on connection
- **WHEN** user connects using a transport (CAN, UART, Fibre, MultiBus, or simulator)
- **THEN** studio probes the active bus for candidate targets and axes and lists them with connection state rather than assuming a single hard-coded target

#### Scenario: Simulated multi-device discovery
- **WHEN** the virtual simulator transport is active
- **THEN** studio exposes at least two simulated targets, each with two axes, so multi-device/multi-axis UI can be exercised without hardware

#### Scenario: Refresh discovery
- **WHEN** user triggers discovery refresh
- **THEN** studio re-probes active transports and adds newly found targets/axes and removes those no longer present without disturbing existing connections

### Requirement: Persistent Axis Status Footer
The studio SHALL render a persistent bottom footer containing one status chip per discovered axis, color-coded for disconnected, healthy, and error states, available on every dashboard and wizard page.

#### Scenario: Footer reflects connection state
- **WHEN** an axis's owning target is disconnected
- **THEN** its footer chip renders in the inactive style and reports no live errors

#### Scenario: Footer reflects error state
- **WHEN** any monitored error field of an axis is non-zero
- **THEN** its footer chip renders in the error style and is visually distinguishable from healthy axes

#### Scenario: Footer persists across views
- **WHEN** user switches between dashboards, wizard, or console views
- **THEN** the footer remains visible with current per-axis states without CSS dot indicators

### Requirement: Human-Readable Error Decoding
Clicking an axis footer chip SHALL open a popup listing axis, motor, encoder, and controller error conditions decoded from bitmasks into human-readable names, plus a Clear Errors control.

#### Scenario: Decoding an error bitmask
- **WHEN** an axis reports a non-zero error bitmask
- **THEN** the popup lists each set error name from the embedded error dictionary instead of only a raw hexadecimal code

#### Scenario: No errors state
- **WHEN** all monitored error fields are zero
- **THEN** the popup displays 'none' for each subsystem

#### Scenario: Clearing errors
- **WHEN** user clicks Clear Errors in the popup
- **THEN** studio writes zero to the axis, motor, encoder, and controller error fields on the target and the footer updates on the next poll

### Requirement: Periodic Error Polling
The studio SHALL poll error fields for connected axes on a fixed interval and update the footer without user interaction.

#### Scenario: Periodic refresh
- **WHEN** an axis is connected and a diagnostic error arises on the target
- **THEN** within one polling interval the footer chip and error popup reflect the new error state
