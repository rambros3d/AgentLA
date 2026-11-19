## ADDED Requirements

### Requirement: ODrive-GUI-Style Studio Shell
The system SHALL present BareFOC Studio with a shell modeled on the ODrive GUI: a header of named dashboards with a menu for dashboard and target-configuration import/export and a global sampling toggle, a dynamic central view area for dashboards/wizard/start pages, and a persistent per-axis status footer.

#### Scenario: Studio shell layout
- **WHEN** user opens `/motor`
- **THEN** studio renders the dashboard header, active view, connection/safety toolbar, and persistent axis footer instead of only the legacy five fixed tab panels

#### Scenario: Sampling toggle
- **WHEN** user clicks start sampling or stop sampling in the header
- **THEN** studio controls live sampling for all registered plot variables

### Requirement: Multi-Device And Multi-Axis Operation
The system SHALL support multiple connected targets, each with multiple axes, and SHALL namespace parameters, dashboards, and status by target and axis.

#### Scenario: Multiple targets visible
- **WHEN** more than one target is discovered
- **THEN** studio lists each target and its axes in the footer and in the parameter tree with unambiguous names

#### Scenario: Target-scoped configuration
- **WHEN** user imports or exports configuration
- **THEN** studio applies or serializes configuration for a specifically selected target rather than a single implicit target

### Requirement: Dynamic Parameter-Driven Controls
The system SHALL generate dashboard controls from the live discovered parameter tree instead of relying only on fixed markup.

#### Scenario: New discovered parameter becomes controllable
- **WHEN** a target exposes a parameter that is not hard-coded in the HTML
- **THEN** user can find it in the parameter tree and add it to a dashboard as an appropriate typed control without source changes

### Requirement: Configuration Of Limit And Modulation Fields
The system SHALL make the modulation scheme, supply voltage, voltage limit, and velocity limit inputs functional by reading and writing corresponding target parameters.

#### Scenario: Reading tuning and hardware settings
- **WHEN** user clicks Read From Target in Controller Tuning
- **THEN** studio reads current gain and limit values from the target and updates the visible controls

#### Scenario: Applying hardware limit changes
- **WHEN** user changes modulation, supply voltage, voltage limit, or velocity limit and applies
- **THEN** studio writes the new value to the target over the active adapter and reflects the stored value

### Requirement: Hardware-Feedback-Driven Alignment
The system SHALL complete electrical-zero alignment based on values parsed from target responses rather than timer-only simulation.

#### Scenario: Real alignment result
- **WHEN** target hardware finishes alignment
- **THEN** studio parses returned zero electrical angle and sensor direction, updates the alignment inputs, and enables Apply & Store Calibration

### Requirement: Functional Target Power Toggle
The system SHALL wire the target power rail toggle to the bridge power API when the platform advertises target power control, and SHALL keep the control guarded when unsupported.

#### Scenario: Power control supported
- **WHEN** the platform reports target power control support and user toggles the power pill
- **THEN** studio calls the bridge power API to enable or disable the rail and updates the pill state

#### Scenario: Power control unsupported
- **WHEN** the platform does not report target power control support
- **THEN** studio keeps the toggle disabled or visibly guarded and does not issue a failing call

## MODIFIED Requirements

### Requirement: Real Binary Fibre RPC Transport
The system SHALL provide an authentic binary Fibre RPC client in `webui/barefoc.html` supporting packet framing (`0xAA` sync byte, payload length, CRC8 header, CRC16 payload), endpoint 0 schema introspection, and typed binary read/write operations for motor telemetry and control.

#### Scenario: Establishing binary Fibre connection and schema request
- **WHEN** client selects Fibre protocol and connects to the target endpoint
- **THEN** client transmits an endpoint 0 read request with framing and CRC, receives the JSON schema, decodes the object tree into normalized typed parameter nodes, and displays available endpoints in the parameter explorer

#### Scenario: Reading and writing typed endpoints
- **WHEN** user adjusts motor velocity or current target or reads telemetry over Fibre
- **THEN** client encodes a binary write or read frame to the target endpoint ID with payload CRC16, and decodes the response payload into numerical values
