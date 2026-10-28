## ADDED Requirements

### Requirement: Operation handles
The system SHALL return an operation handle for queued or long-running command requests.

#### Scenario: Long command
- **WHEN** a client requests a queued flash, capture, or erase operation
- **THEN** the response SHALL include an operation handle and initial state

### Requirement: Progress events
The system SHALL expose operation progress through SSE as structured events keyed by operation handle.

#### Scenario: Flash progress
- **WHEN** a long command advances
- **THEN** SSE SHALL emit bytes completed, total bytes, state, and job ID

### Requirement: Bounded transport
The system SHALL reject oversized WebUI payload or claim-request arguments with a documented error.

#### Scenario: Oversize request
- **WHEN** request payload exceeds its maximum configured size
- **THEN** the system SHALL return a payload-size JSON-RPC error before dispatching command execution