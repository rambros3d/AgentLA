## ADDED Requirements

### Requirement: BareFoc Playbook Resource
The MCP server SHALL advertise and serve a static resource `agentla://skills/barefoc` containing a BareFOC agentic debugging playbook.

#### Scenario: Resource is listed
- **WHEN** an agent calls `resources/list`
- **THEN** the response contains `agentla://skills/barefoc` with a human-readable name and description

#### Scenario: Resource is read
- **WHEN** an agent calls `resources/read` with `agentla://skills/barefoc`
- **THEN** the server returns markdown playbook content covering safety first, target discovery, telemetry, register access, command/mode control, calibration, error decoding, and safe-state teardown

### Requirement: BareFoc Debug Prompt
The MCP server SHALL advertise a `barefoc_debug` prompt with an optional `node` argument that guides an agent through a safe debugging workflow for an external FOC target.

#### Scenario: Prompt is listed
- **WHEN** an agent calls `prompts/list`
- **THEN** the response contains the `barefoc_debug` prompt with its optional `node` argument

#### Scenario: Prompt is retrieved
- **WHEN** an agent calls `prompts/get` for `barefoc_debug`
- **THEN** the server returns a text prompt that starts with safety checks and proceeds through discovery, telemetry, tuning, and safe teardown using the BareFOC tools
