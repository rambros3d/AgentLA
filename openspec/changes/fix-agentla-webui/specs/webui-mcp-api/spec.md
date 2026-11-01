## ADDED Requirements

### Requirement: Granular long-running job classification
The command bridge SHALL classify only bulk, high-latency commands (`FLASH ERASE`, `MASSERASE`, bulk `CAPTURE`) as queued jobs, while keeping per-chunk write and read operations (`FLASH WRITE`, `FLASH VERIFY`, `FLASH READ`) synchronous.

#### Scenario: Synchronous chunk write
- **WHEN** a client sends `cmd` with `name=FLASH WRITE`
- **THEN** the bridge executes the write synchronously and immediately returns `result.stdout` containing `OK FLASH WROTE` without enqueuing a background job

#### Scenario: Queued bulk erase
- **WHEN** a client sends `cmd` with `name=FLASH ERASE`
- **THEN** the bridge enqueues a background job and returns `result.job_id` with `queued: true`

## MODIFIED Requirements

### Requirement: JSON-RPC command bridge
The system SHALL accept JSON-RPC 2.0-style `cmd` requests over `/mcp` with `id`, `method`, and `params`, and SHALL return a JSON-RPC response containing the original `id`. The client-side `callMcp()` utility SHALL parse the response JSON and expose structured `stdout`, `status`, and `error` fields to caller modules.

#### Scenario: Successful command
- **WHEN** a client sends `{"jsonrpc":"2.0","id":7,"method":"cmd","params":{"name":"IDENTIFY"}}`
- **THEN** the response contains the same `id` and a command result containing `stdout` and execution status

#### Scenario: Invalid request
- **WHEN** a request lacks `method` or `id`
- **THEN** the response SHALL contain a JSON-RPC error object and no command execution
