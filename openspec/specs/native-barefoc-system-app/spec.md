# native-barefoc-system-app Specification

## Purpose
TBD - created by archiving change native-barefoc-system-app. Update Purpose after archive.
## Requirements
### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/barefoc.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving barefoc asset from SYSTEM_FS
- **WHEN** client requests `GET /barefoc.html` or `GET /motor`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/motor` to `SystemWebFile::new("barefoc.html", "text/html")` and provide HTTP redirects for `/foc` and `/barefoc`.

#### Scenario: Accessing canonical motor route
- **WHEN** user navigates to `/motor`
- **THEN** server responds with HTTP 200 and serves BareFOC Studio

#### Scenario: Accessing motor alias routes
- **WHEN** user navigates to `/foc` or `/barefoc`
- **THEN** server responds with HTTP 301 redirect to `/motor`

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `Motor` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope` or `/terminal`
- **THEN** the top navigation rail displays a link to `Motor` (`/motor`)

#### Scenario: Active state styling on motor page
- **WHEN** user is viewing `/motor`
- **THEN** the Motor navigation item is highlighted with surface active styling without CSS dot indicators

### Requirement: Dock and Application Drawer Integration
The system SHALL integrate the BareFOC Studio launcher into the macOS-style dock and desktop application drawer.

#### Scenario: Launching from dock
- **WHEN** user clicks the Motor icon in the glass dock
- **THEN** the system navigates directly to `/motor`

### Requirement: WebUI Packaging Inclusion
The build utility `tools/pack_webui.py` SHALL include `barefoc.html` when packaging `SYSTEM_FS` image and update tarballs.

#### Scenario: Generating system update tarball
- **WHEN** `pack_webui.py` builds the update tarball
- **THEN** `barefoc.html` and its pre-compressed `barefoc.html.gz` are embedded in the output archive

### Requirement: Real Binary Fibre RPC Transport
The system SHALL provide an authentic binary Fibre RPC client in `webui/barefoc.html` supporting packet framing (`0xAA` sync byte, payload length, CRC8 header, CRC16 payload), endpoint 0 schema introspection, and typed binary read/write operations for motor telemetry and control.

#### Scenario: Establishing binary Fibre connection and schema request
- **WHEN** client selects Fibre protocol and connects to the target endpoint
- **THEN** client transmits an endpoint 0 read request with framing and CRC, receives the JSON schema, decodes the object tree, and displays available endpoints in the UI

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

