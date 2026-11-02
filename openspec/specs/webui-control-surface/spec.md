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
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, and manage two primary top-level modes (`Active Bench` and `Target Station`) alongside the hardware pin drawer.

#### Scenario: Successful WebUI boot
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** `#loading-screen` receives the `fade-out` class within 500ms and the top bar displays `[ ⚡ Active Bench ]` as the default active mode

#### Scenario: Switching primary modes
- **WHEN** the user clicks `#nav-target-station` or `#nav-active-bench`
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

### Requirement: Live connection state tracking
The WebUI SHALL periodically poll or verify probe connectivity via `/mcp` and update the connection indicator accordingly.

#### Scenario: Active connection state
- **WHEN** a probe heartbeat query succeeds
- **THEN** the status indicator displays "Connected" with a green status dot

