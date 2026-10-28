## ADDED Requirements

### Requirement: JSON-RPC command bridge
The system SHALL accept JSON-RPC 2.0-style `cmd` requests over `/mcp` with `id`, `method`, and `params`, and SHALL return a JSON-RPC response containing the original `id`.

#### Scenario: Successful command
- **WHEN** a client sends `{"jsonrpc":"2.0","id":7,"method":"cmd","params":{"name":"IDENTIFY"}}`
- **THEN** the response contains the same `id` and a command result

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
