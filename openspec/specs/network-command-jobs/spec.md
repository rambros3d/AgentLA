# network-command-jobs Specification

## Purpose
TBD - created by syncing change webui-mcp-command-bridge. Update Purpose after archive.

## Requirements
### Requirement: Long-operation jobs
The system SHALL support operations that return immediately with an operation handle and continue asynchronously.

#### Scenario: Job accepted
- **WHEN** a client requests a long operation such as `FLASH`, `CAPTURE`, or `MASSERASE`
- **THEN** the response SHALL include a handle, initial state, and a progress stream identity

#### Scenario: Job status query
- **WHEN** a client queries `jobs/status` with a valid handle
- **THEN** the system SHALL return the current or terminal state

### Requirement: Operation streaming events
The system SHALL emit progress and terminal events that reference the operation handle.

#### Scenario: Flash progress
- **WHEN** a queued flash operation advances
- **THEN** an SSE event SHALL include the handle, bytes completed, total bytes, and a done flag

#### Scenario: Lost client
- **WHEN** the SSE client disconnects and later reconnects
- **THEN** the system SHALL either replay or mark events lost without corrupting the operation result

### Requirement: Bounded request payloads
The system SHALL enforce bounded JSON-RPC payload and base64 chunk sizes and report a clear error when exceeded.

#### Scenario: Oversize request
- **WHEN** request payload exceeds the supported limit
- **THEN** the system SHALL return a payload-too-large JSON-RPC error before execution