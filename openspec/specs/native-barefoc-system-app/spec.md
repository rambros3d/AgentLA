# native-barefoc-system-app Specification

## Purpose
TBD - created by archiving change native-barefoc-system-app. Update Purpose after archive.
## Requirements
### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/barefoc.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving barefoc asset from SYSTEM_FS
- **WHEN** client requests `GET /barefoc.html` or `GET /barefoc`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/barefoc` to `SystemWebFile::new("barefoc.html", "text/html")` as the canonical route and provide HTTP 301 redirects for `/motor`, `/motor/`, `/foc`, `/foc/`, and `/barefoc/` to `/barefoc`.

#### Scenario: Accessing canonical barefoc route
- **WHEN** user navigates to `/barefoc`
- **THEN** server responds with HTTP 200 and serves BareFOC Studio

#### Scenario: Accessing alias routes
- **WHEN** user navigates to `/motor`, `/foc`, or `/barefoc/`
- **THEN** server responds with HTTP 301 redirect to `/barefoc`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `BareFOC` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope` or `/terminal`
- **THEN** the top navigation rail displays a link to `BareFOC` (`/barefoc`)

#### Scenario: Active state styling on barefoc page
- **WHEN** user is viewing `/barefoc`
- **THEN** the BareFOC navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration
The system SHALL integrate the BareFOC Studio launcher into the macOS-style dock and desktop application drawer.

#### Scenario: Launching from dock
- **WHEN** user clicks the BareFOC icon in the glass dock
- **THEN** the system navigates directly to `/barefoc`

### Requirement: WebUI Packaging Inclusion
The build utility `tools/pack_webui.py` SHALL include `barefoc.html` when packaging `SYSTEM_FS` image and update tarballs.

#### Scenario: Generating system update tarball
- **WHEN** `pack_webui.py` builds the update tarball
- **THEN** `barefoc.html` and its pre-compressed `barefoc.html.gz` are embedded in the output archive

### Requirement: Real Binary Fibre RPC Transport
The system SHALL provide an authentic binary Fibre RPC client in `webui/barefoc.html` supporting packet framing (`0xAA` sync byte, payload length, CRC8 header, CRC16 payload), endpoint 0 schema introspection, and typed binary read/write operations for motor telemetry and control.

#### Scenario: Establishing binary Fibre connection and schema request
- **WHEN** client selects Fibre protocol and connects to the target endpoint
- **THEN** client transmits an endpoint 0 read request with framing and CRC, receives the JSON schema, decodes the object tree into normalized typed parameter nodes, and displays available endpoints in the parameter explorer

#### Scenario: Reading and writing typed endpoints
- **WHEN** user adjusts motor velocity or current target or reads telemetry over Fibre
- **THEN** client encodes a binary write or read frame to the target endpoint ID with payload CRC16, and decodes the response payload into numerical values

### Requirement: Hardware CAN Bus Transport
The system SHALL provide live CAN bus communication in `webui/barefoc.html` over AgentLA's active WebSocket bridge (`/api/v1/can/ws`), supporting both SimpleFOC CAN and ODrive CAN frame formats.

#### Scenario: Connecting to AgentLA CAN WebSocket bridge
- **WHEN** user selects CAN Bus transport and connects
- **THEN** client opens a WebSocket to `/api/v1/can/ws`, parses incoming CAN frames, and displays bus activity

#### Scenario: Multiplexing SimpleFOC and ODrive CAN messages
- **WHEN** user sets target velocity or angle or receives heartbeat frames over CAN
- **THEN** client formats and transmits arbitration IDs matching the selected standard (11-bit SimpleFOC or 6-bit node ID + 5-bit command ID ODrive CAN), and filters incoming frames by node ID

### Requirement: Hardware MultiBus Bridge Transport
The system SHALL connect the `MultiBusAdapter` in `webui/barefoc.html` to AgentLA's hardware bus bridge endpoints, allowing read and write access to physical target control tables over I2C or SPI.

#### Scenario: Reading target registers via AgentLA bus bridges
- **WHEN** user inspects registers over MultiBus
- **THEN** client dispatches bus read commands to the target address via `/api/v1/bridge/i2c` or MCP tool APIs and populates the control table view with live hardware values

#### Scenario: Writing target registers via AgentLA bus bridges
- **WHEN** user modifies target velocity, voltage limit, or control mode over MultiBus
- **THEN** client dispatches register write packets containing register address and payload bytes to the target device

### Requirement: Hardware-Driven Alignment Wizard
The system SHALL provide an asynchronous, hardware-driven alignment sequence in `webui/barefoc.html` that sends real SimpleFOC alignment commands (`M1` alignment routines) and awaits physical hardware feedback instead of using simulated timers.

#### Scenario: Executing asynchronous SimpleFOC calibration commands
- **WHEN** user clicks "Start Alignment" in the Hardware Alignment Wizard
- **THEN** client dispatches SimpleFOC alignment commands over the active transport, updates step indicators based on actual target responses, and does not rely on mock timer intervals

#### Scenario: Parsing physical zero electrical angle response
- **WHEN** target hardware finishes pole pair and sensor alignment
- **THEN** client parses the returned `zero_electric_angle` float and sensor direction, updates the calibration inputs, and saves the configuration

### Requirement: Gate Driver Register Configuration
The system SHALL support reading and writing gate driver SPI registers (TI DRV8316 and DRV8311) via the active hardware bridge or serial interface.

#### Scenario: Inspecting gate driver registers
- **WHEN** user opens the Gate Driver tab and requests register status
- **THEN** client reads the control and fault status registers over SPI/bridge and reflects IC state in the register card

#### Scenario: Updating gate driver configuration
- **WHEN** user modifies gate driver parameters (current limit, slew rate, PWM mode) and applies changes
- **THEN** client writes the updated register bitfields over SPI/bridge and verifies register write acknowledgment

### Requirement: Responsive Two-Column Landscape Layout
The system SHALL present the BareFOC Studio dashboard (`#view-dashboard`) in a responsive two-column grid on wide screens (`lg:` breakpoint and above), arranging live telemetry and phase vector diagnostics in the left column and motion control modes and the radial setpoint dial in the right column to fit within a single landscape viewport without vertical scrolling.

#### Scenario: Viewing dashboard on wide landscape display
- **WHEN** user opens BareFOC Studio on a wide display (viewport width >= 1024px)
- **THEN** the dashboard renders with live telemetry and phase vector diagnostics in the left 5 columns and motion modes and the interactive radial dial in the right 7 columns, fitting the viewport without vertical scrolling

#### Scenario: Viewing dashboard on mobile or narrow display
- **WHEN** user opens BareFOC Studio on a narrow display (viewport width < 1024px)
- **THEN** the dashboard collapses into a single vertical column preserving full functional accessibility

### Requirement: ODrive-GUI-Style Studio Shell
The system SHALL present BareFOC Studio with a shell modeled on the ODrive GUI: a header of named dashboards with a menu for dashboard and target-configuration import/export and a global sampling toggle, a dynamic central view area for dashboards/wizard/start pages, and a persistent per-axis status footer.

#### Scenario: Studio shell layout
- **WHEN** user opens `/barefoc`
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

