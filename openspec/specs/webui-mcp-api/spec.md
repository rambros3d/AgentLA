# webui-mcp-api Specification

## Purpose
TBD
## Requirements
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

### Requirement: JSON-RPC command bridge
The system SHALL accept JSON-RPC 2.0-style `cmd` requests over `/mcp` with `id`, `method`, and `params`, and SHALL return a JSON-RPC response containing the original `id`. The client-side `callMcp()` utility SHALL parse the response JSON and expose structured `stdout`, `status`, and `error` fields to caller modules.

#### Scenario: Successful command
- **WHEN** a client sends `{"jsonrpc":"2.0","id":7,"method":"cmd","params":{"name":"IDENTIFY"}}`
- **THEN** the response contains the same `id` and a command result containing `stdout` and execution status

#### Scenario: Invalid request
- **WHEN** a request lacks `method` or `id`
- **THEN** the response SHALL contain a JSON-RPC error object and no command execution

### Requirement: Command routing from request parameters
The system SHALL assemble request parameters into the existing command syntax, invoke the command dispatcher, and return command stdout in the result.

#### Scenario: I2C claim and read
- **WHEN** a client invokes `cmd` with `name=I2C_READ` and token string parameters
- **THEN** the firmware SHALL invoke the existing `I2C_READ` behavior and include its response text in the JSON result

#### Scenario: Unknown command
- **WHEN** `params.name` is not a valid host-facing command name
- **THEN** the system SHALL return a typed JSON-RPC error and SHALL NOT route it to internal command execution

### Requirement: Granular long-running job classification
The command bridge SHALL classify only bulk, high-latency commands (`FLASH ERASE`, `MASSERASE`, bulk `CAPTURE`) as queued jobs, while keeping per-chunk write and read operations (`FLASH WRITE`, `FLASH VERIFY`, `FLASH READ`) synchronous.

#### Scenario: Synchronous chunk write
- **WHEN** a client sends `cmd` with `name=FLASH WRITE`
- **THEN** the bridge executes the write synchronously and immediately returns `result.stdout` containing `OK FLASH WROTE` without enqueuing a background job

#### Scenario: Queued bulk erase
- **WHEN** a client sends `cmd` with `name=FLASH ERASE`
- **THEN** the bridge enqueues a background job and returns `result.job_id` with `queued: true`

