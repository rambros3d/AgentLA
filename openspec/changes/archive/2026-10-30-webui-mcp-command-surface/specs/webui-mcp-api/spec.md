## ADDED Requirements

### Requirement: Browser command discovery
The system SHALL report allowed command names in grouped safety classes.

#### Scenario: Public command list
- **WHEN** a client sends `GET /mcp?method=list`
- **THEN** the response SHALL include read-only, setup, adapter, risky, and hidden command classes

### Requirement: Command request parsing
The system SHALL parse `id`, `method`, `name`, and `args` fields and execute canonical command strings through firmware dispatch.

#### Scenario: Command request
- **WHEN** a client sends a valid command request
- **THEN** firmware SHALL execute the command and include its stdout, status, and input id in a JSON response

### Requirement: Command safety classes
The system SHALL reject exposed diagnostic and self-test command names unless an explicit unsafe mode is enabled.

#### Scenario: Hidden method
- **WHEN** a request names a diagnostics-only command
- **THEN** the system SHALL return a safety-class JSON error without invoking it