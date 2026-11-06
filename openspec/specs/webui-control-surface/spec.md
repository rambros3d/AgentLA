# webui-control-surface Specification

## Purpose
TBD - created by syncing change webui-mcp-command-bridge. Update Purpose after archive.
## Requirements
### Requirement: Form-defined operation inputs
The WebUI SHALL transmit values entered in Flasher, Debugger, Adapter, Capture, and GPIO dialog fields rather than discarding or ignoring them.

#### Scenario: Flash file selection
- **WHEN** a user selects a target file and starts flashing
- **THEN** the UI SHALL send the file bytes or encoded chunks with a validated size to `/mcp`

#### Scenario: Read memory fields
- **WHEN** a user enters a memory address and length in the Debugger window and requests a read
- **THEN** the request SHALL dispatch `DEBUG READ <addr>` to inspect target memory rather than storage flash

### Requirement: Tool visibility by safety class
The WebUI SHALL expose command classes so that destructive operations require confirmation and exact inputs.

#### Scenario: Destructive operation preview
- **WHEN** a user initiates `MASSERASE`, `RDPUNLOCK`, or `FLASH` over the WebUI
- **THEN** the interface SHALL display the operation, arguments, and warning before dispatch

### Requirement: SSE-driven progress displays
The WebUI SHALL update progress displays from namespaced SSE events rather than timers or simulated progress.

#### Scenario: Flash completion
- **WHEN** SSE reports completion
- **THEN** the progress bar SHALL show complete only after receiving the terminal event

### Requirement: WebUI lifecycle initialization & Start menu control
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, load `/board.json`, and dynamically initialize the top-level navigation, board branding, and mode controls based on the probe's declared hardware profile.

#### Scenario: Successful WebUI boot
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** `#loading-screen` receives the `fade-out` class within 500ms, the board branding header displays the board's name and `logo.svg`, and the top bar displays `[ ⚡ Active Bench ]` as the default active mode

#### Scenario: Switching primary modes
- **WHEN** the user clicks `#nav-target-station` or `#nav-active-bench`
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

### Requirement: Live connection state tracking
The WebUI SHALL periodically poll or verify probe connectivity via `/mcp` and update the connection indicator accordingly.

#### Scenario: Active connection state
- **WHEN** a probe heartbeat query succeeds
- **THEN** the status indicator displays "Connected" with a green status dot

### Requirement: Open Source Project Link in Header
The WebUI SHALL display an Open Source Initiative (OSI) keyhole icon button immediately to the left of the theme toggle in the top application bar, linking to the project documentation site.

#### Scenario: User navigates to Open Source site
- **WHEN** user clicks the OSI icon button (`#btn-open-source`) in the header
- **THEN** the browser opens `https://rambros3d.github.io/AgentLA` in a new browser tab (`target="_blank" rel="noopener noreferrer"`) without interrupting active bench measurements or captures

### Requirement: Console Tab Agent Guide Modal
The Console view SHALL provide an "Agent Guide" button (`#btn-agent-guide`) in its header actions that opens an interactive modal dialog presenting unified MCP setup instructions, transport endpoints, tools overview, and sample prompts for autonomous AI coding agents (Antigravity, Claude, OpenCode, Cline, Cursor).

#### Scenario: User opens Agent Guide
- **WHEN** user clicks `#btn-agent-guide` in the Console header
- **THEN** `#modal-agent-guide` opens displaying universal MCP configuration instructions and a copyable JSON snippet

#### Scenario: User copies universal MCP configuration
- **WHEN** user clicks `#btn-copy-mcp-config` inside the Agent Guide modal
- **THEN** the universal `mcpServers.agentla` JSON snippet is copied to clipboard and the button text temporarily confirms "Copied!"

### Requirement: Console Text Wrapping & Viewport Containment
The Console terminal window SHALL wrap long unbreakable words, hex dumps, and serialized JSON without forcing the container or page to overflow horizontally beyond the viewport.

#### Scenario: Rendering long unbreakable hex or JSON response
- **WHEN** the console receives a response string longer than the terminal container width without spaces
- **THEN** the text breaks onto the next line within `.panel.terminal-display` via `overflow-wrap: anywhere; word-break: break-word;` and `.console-main-grid` maintains strict viewport boundaries

### Requirement: Console Semantic Theming and Click-to-Copy
The Console output display SHALL format command inputs, standard echoes, success results, and errors using semantic theme tokens, and SHALL copy individual lines to the system clipboard upon click.

#### Scenario: Differentiating command input from output
- **WHEN** a user or agent submits a command
- **THEN** the prompt and command text renders in `var(--color-primary)` and responses render in `var(--color-base-content)` (or `var(--color-success)` / `var(--color-error)`)

#### Scenario: Clicking an output line to copy
- **WHEN** a user clicks on any output line in `#cmd-output-content` without performing a drag-selection
- **THEN** the line text is copied to the clipboard and a brief inline "Copied!" badge is displayed

### Requirement: Terminal Output Clear Badge Button
The Console terminal window (`.panel.terminal-display`) SHALL provide an integrated floating clear badge button (`#btn-cmd-clear`) in its bottom-right corner that clears displayed log output upon click.

#### Scenario: User clears console output
- **WHEN** user clicks `#btn-cmd-clear` located in the bottom-right corner of the terminal window
- **THEN** `#cmd-output-content` text is cleared and the terminal view resets to blank

### Requirement: Top-Bar Target Power Toggle Pill
The WebUI top application header SHALL display a live Target Power pill button showing measured voltage and current telemetry, enabling single-click power rail toggling across all workbench views.

#### Scenario: Displaying live power rail telemetry
- **WHEN** the target power rail is enabled
- **THEN** the top-bar power pill displays an active bolt icon, soft green background, and real-time voltage and current values (e.g., `3.31V · 14mA`)

#### Scenario: Toggling power rail state
- **WHEN** the user clicks the top-bar power pill while the rail is enabled
- **THEN** the WebUI dispatches `POWER OFF` over `/mcp` and updates the pill to standby state (`POWER OFF` with muted dot)

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

### Requirement: Real-Time Adapter Protocol Terminal Logging
The WebUI Adapter view SHALL route all outgoing bus transmissions (UART, I2C, SPI, CAN, Modbus) directly into the protocol's dedicated terminal log window via `appendAdapterLog()`, displaying timestamped direction indicators, hex bytes, and ASCII interpretations.

#### Scenario: Transmitting I2C or SPI write
- **WHEN** the user transmits an I2C or SPI frame from the Adapter tab
- **THEN** the corresponding `#i2c-log-content` or `#spi-log-content` terminal immediately appends a formatted entry displaying `[TX]` timestamp and data bytes
- **AND** the terminal view automatically scrolls to the newest log line.

#### Scenario: Auto-sniff capture notification
- **WHEN** bus transmission initiates with auto-sniff enabled
- **THEN** the adapter terminal displays an informational entry noting capture arming and channel assignment.

### Requirement: Dynamic Desktop Sidebar Navigation Tabs
The desktop vertical navigation sidebar (`.desktop-vertical-nav`) SHALL provide dynamic icon/text tabs matching the mobile bottom navigation pattern, where inactive tabs display only their icon and the active tab dynamically displays both icon and text label.

#### Scenario: Inactive desktop sidebar tab presentation
- **WHEN** a navigation mode button in the desktop sidebar is not active
- **THEN** it displays its icon centered in a compact button footprint with its label collapsed (`opacity: 0`).

#### Scenario: Active desktop sidebar tab presentation
- **WHEN** a navigation mode button becomes active
- **THEN** it dynamically expands to reveal both icon and text label with smooth transition and theme active highlight.

### Requirement: Pin Matrix Instant Action Button Styling
The Pin Matrix view SHALL style instant GPIO control buttons (`.btn-pin-act` for High `H`, Low `L`, Pull-Up `PU`, Pull-Down `PD`, and High-Z `Z`) using compact DaisyUI button group aesthetics with clear active/pressed states.

#### Scenario: Visual presentation of pin action buttons
- **WHEN** the Pin Matrix table renders GPIO pin rows
- **THEN** each row displays a compact button group containing `H`, `L`, `PU`, `PD`, and `Z` buttons styled with theme tokens (`--color-base-200`, `--color-primary`, `--radius-field`)
- **AND** the active pin mode button displays a distinct active accent highlight.

### Requirement: Unified Semantic Color Tokens
The WebUI stylesheets SHALL utilize semantic DaisyUI theme tokens (`var(--color-primary)`, `var(--color-base-100)`, `var(--color-base-content)`, etc.) and purge deprecated hardcoded color tokens (`var(--accent-cyan)`).

#### Scenario: Dynamic theme adaptability of workbench elements
- **WHEN** switching themes
- **THEN** borders, focus rings, headers, tab active highlights, and indicators adapt seamlessly to the new theme's primary and base colors without residual cyan tints.

