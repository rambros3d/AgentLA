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
- **WHEN** a user enters an address and length
- **THEN** the request SHALL include those values and the UI SHALL display the corresponding bytes

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