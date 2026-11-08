# rom-device-diagnostics Specification

## Purpose
Provides a standalone, self-contained Flash ROM diagnostics and recovery interface (`device.html`) on `/device` and `/device.html`. The page operates with zero dependencies on LittleFS partitions, presenting live board telemetry, dual-slot A/B status, a browser-mediated update checker with independent firmware and WebUI flashers, a visual Flash partition map, dedicated `SYSTEM_FS` and `USER_FS` storage controls, and an in-page Neo-Industrial confirmation modal for developer recovery tools (hands-free BOOTSEL reboot, pin arbiter emergency releases, pin probing, and partition formatting).

## Requirements

### Requirement: ROM-Embedded Standalone Diagnostics Page
The system SHALL embed the complete HTML, CSS, and JavaScript source of `device.html` directly into the firmware Flash binary via `include_str!` or `include_bytes!`, serving it with `Content-Type: text/html` and `Cache-Control: no-cache, must-revalidate` without accessing LittleFS storage, and cross-linking with the interactive fallback recovery interface.

#### Scenario: Navigating to /device when LittleFS is unformatted or corrupted
- **WHEN** a client navigates to `http://10.10.10.10/device` or `http://10.10.10.10/device.html` while `SYSTEM_FS` is completely unformatted or corrupt
- **THEN** the server returns HTTP 200 OK and serves the complete diagnostics page with intact styling and scripts.

#### Scenario: Navigating from recovery fallback to diagnostics
- **WHEN** user clicks the diagnostics button on the uninitialized partition recovery page
- **THEN** the browser opens `/device` with full hands-free developer tools available.

### Requirement: Detailed Board & Hardware Capability Reporting
The diagnostics page SHALL query `/api/board/manifest` and `/api/v1/system/telemetry` to display MCU details (chip model, cores, clock speed, flash/SRAM geometry), power metrics (V_SYS, V_TARGET, current sense, core temperature), and the complete 30-pin Dynamic Pin Arbiter ownership matrix.

#### Scenario: Displaying live hardware metrics
- **WHEN** the diagnostics page loads in a client browser
- **THEN** it renders MCU specifications, supply voltages, current consumption, temperature, and a visual matrix of GPIO 0 to 29 showing whether each pin is `free` or claimed by an active subsystem.

### Requirement: Visual Flash Partition Map & Dedicated Partition Cards
The diagnostics page SHALL display a proportional segmented linear memory bar and geometry table representing the Flash layout (`BOOT_TABLE`, `FW_SLOT_A`, `FW_SLOT_B`, `SYSTEM_FS`, `USER_FS`, `TRAIL`), with dedicated cards for `WebUI & Storage (SYSTEM_FS)` and `User Storage (USER_FS)` containing in-place formatting controls.

#### Scenario: Viewing memory geometry and partition state
- **WHEN** the diagnostics page loads
- **THEN** it highlights the currently active firmware execution slot in the segmented linear bar and displays offset boundaries and LittleFS headroom for both `SYSTEM_FS` (512 KB at `0x00204000`) and `USER_FS` (1504 KB at `0x00284000`).

### Requirement: In-Page Action Confirmation Modal
Destructive developer actions (reboot, BOOTSEL entry, slot rollback, pin release, partition format) SHALL prompt the user via an in-page Neo-Industrial confirmation modal dialog rather than standard browser dialogs (`confirm()` / `alert()`).

#### Scenario: Triggering a destructive partition format or reboot
- **WHEN** the user clicks a destructive action button
- **THEN** an in-page modal dialog appears with an action description and high-contrast `[ CANCEL ]` and `[ EXECUTE ACTION ]` buttons, awaiting explicit user confirmation before issuing the REST call.

### Requirement: Unified System Update Hero Card in Hardware View
The In-ROM diagnostics console (`device.html`) SHALL render a full-width System Update Hero Card (`#cardSystemUpdate`) positioned directly at the top of the `[ HARDWARE ]` view panel. The card SHALL present a unified release management interface combining firmware dual-slot status, WebUI asset status, GitHub release version comparison, a single-click 2-phase full system update runner, manual local file staging pickers, and firmware rollback.

#### Scenario: Displaying unified system update card on hardware tab
- **WHEN** the user opens `http://<host>/device`
- **THEN** the console displays `#cardSystemUpdate` at the top of the Hardware panel displaying installed vs latest versions for both Firmware and WebUI alongside a unified `[ AUTO UPDATE ALL ]` button.

#### Scenario: Handling unpopulated WebUI state without separate recovery card
- **WHEN** `SYSTEM_FS` is blank, unformatted, or contains 0 used bytes
- **THEN** the unified card cleanly displays `WebUI: Not Installed` and provides 1-click cloud restoration (`Auto Update`) and local drag-and-drop `.tar` staging, with no secondary `#recoveryBanner` rendered.

### Requirement: Dedicated Storage Tab and Subpath Routing
The In-ROM diagnostics console (`device.html`) SHALL provide a dedicated `[ STORAGE ]` tab (`#tabStorage`) and view panel (`#viewStorage`) positioned immediately adjacent to `[ HARDWARE ]`. The storage panel SHALL host the `WebUI & Storage (SYSTEM_FS)` card, `User Storage (USER_FS)` card, and the full-width Flash Partitions linear memory bar and geometry table. The HTTP router SHALL register `/device/storage` and `/device/storage/` routing directly to this view.

#### Scenario: Navigating to /device/storage
- **WHEN** user navigates to `http://<host>/device/storage` or clicks the `[ STORAGE ]` tab
- **THEN** the browser opens the Storage view panel displaying the LittleFS partition cards, in-place partition formatting controls, and the flash geometry table without showing MCU silicon or telemetry cards.

### Requirement: In-Browser Update Checker with Individual Flashers
The diagnostics page SHALL provide an in-browser update checker that queries the GitHub Releases API (`api.github.com/repos/rambros3d/AgentLA/releases/latest`) using the client's internet connection, compares installed versions against remote releases, and provides both an atomic full-system 2-phase update and distinct, separate update actions for Firmware and WebUI.

#### Scenario: Detecting update availability from official repository
- **WHEN** the user opens the diagnostics page while connected to the internet
- **THEN** client JavaScript queries `https://api.github.com/repos/rambros3d/AgentLA/releases/latest`, compares the installed firmware version (from `/api/v1/system/firmware/status`) and WebUI asset manifest against the latest GitHub release, and updates the release tag badge and update status.

#### Scenario: Individual firmware update
- **WHEN** the user initiates a firmware update via 1-click download or file picker
- **THEN** the client streams the firmware binary to `POST /api/v1/system/firmware/ota`, displays upload progress, and prompts to reboot into the candidate slot.

#### Scenario: Individual WebUI update
- **WHEN** the user initiates a WebUI update via 1-click download or file picker
- **THEN** the client streams the update tarball to `POST /api/v1/system/webui/update`, displays extraction progress, and updates `SYSTEM_FS` without requiring a firmware reboot.

### Requirement: Multi-View Tab Navigation and Direct Subpath Routing
The In-ROM diagnostics console (`device.html`) SHALL provide a top-level tab switcher supporting dedicated view panels: `[ HARDWARE ]` (`#hardware`), `[ STORAGE ]` (`#storage`), `[ PIN DIAGNOSTICS ]` (`#pins`), `[ REST API ]` (`#api`), and `[ MCP AGENT ]` (`#mcp`). The firmware HTTP router SHALL register route aliases for `/device/storage`, `/device/pins`, `/device/api`, `/device/mcp` and their slash variants serving the compiled Flash ROM template, and the client JavaScript SHALL synchronize the active tab automatically with the URL path or hash without triggering a full page reload.

#### Scenario: Navigating directly to /device/mcp
- **WHEN** an operator or AI agent navigates to `http://10.10.10.10/device/mcp`
- **THEN** the server returns HTTP 200 OK serving `device.html` and client JavaScript automatically activates the `[ MCP AGENT ]` tab panel.

#### Scenario: Switching tabs in the browser
- **WHEN** the user clicks the `[ REST API ]` tab header while on the Hardware view
- **THEN** the hardware panel hides, the REST API catalog displays immediately in 0 ms, and the browser URL hash updates to `#api`.

### Requirement: Interactive REST API Catalog and Endpoint Runner
The diagnostics console SHALL provide a categorized reference table of all native firmware endpoints (System Telemetry, Pin Arbiter, Peripheral Transceivers, Servo/PWM, Firmware Management, and WebSockets). Each entry SHALL display the HTTP method badge, route path, description, a copyable `curl` snippet, and a live "Try Endpoint" runner allowing developers to test endpoints with live JSON response feedback directly in the browser.

#### Scenario: Copying a curl command for telemetry
- **WHEN** the user views `/api/v1/system/telemetry` in the REST API tab and clicks "Copy curl"
- **THEN** a complete `curl -s http://<host>/api/v1/system/telemetry` command is copied to the clipboard with an inline confirmation indicator.

#### Scenario: Executing a live API query from the console
- **WHEN** the user clicks "Try Endpoint" on `/api/board/manifest`
- **THEN** the console issues an in-browser `fetch()` call and displays the formatted JSON response and HTTP status code in an adjacent response viewer.

### Requirement: MCP Agent Bridge Integration and Schema Reference
The diagnostics console SHALL provide a dedicated MCP Agent view documenting the probe's Model Context Protocol server. The view SHALL state the HTTP endpoint (`/mcp`) and SSE stream (`/mcp/sse`), provide a 1-click copyable `mcpServers.agentla` configuration snippet for Google Antigravity, Anthropic Claude Desktop, and Cursor/OpenCode, and list all native on-probe tools with their JSON-RPC method names, input schemas, and functional descriptions.

#### Scenario: Copying AI agent MCP configuration
- **WHEN** the user clicks "Copy Config" in the MCP Agent tab
- **THEN** a validated JSON snippet containing `"agentla": { "url": "http://<host>/mcp" }` is copied to the system clipboard.

#### Scenario: Inspecting on-probe MCP tools
- **WHEN** viewing the MCP Agent panel
- **THEN** the console enumerates `mcp_target_diagnose`, `mcp_inspect_firmware`, `mcp_sniff_bus`, `mcp_pins_claim`, and `mcp_telemetry_get` alongside their respective argument parameters.
