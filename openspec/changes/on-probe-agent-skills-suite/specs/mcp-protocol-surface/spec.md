## ADDED Requirements

### Requirement: MCP Agent Skills as Resources
The MCP server SHALL list on-probe agent skills under `resources/list` with URIs matching `agentla://skills/{skill_name}` and return skill markdown manifests via `resources/read`.

#### Scenario: List skills resources
- **WHEN** client sends `resources/list`
- **THEN** response includes `agentla://skills/dev`, `agentla://skills/debug`, `agentla://skills/programmer`, and `agentla://skills/analyzer` with `application/octet-stream` or `text/markdown` MIME types.

#### Scenario: Read skill resource content
- **WHEN** client sends `resources/read` with URI `agentla://skills/dev`
- **THEN** response returns the structured playbook text describing the autonomous development workflow.

### Requirement: MCP Prompts Protocol Support
The MCP server SHALL support `prompts/list` and `prompts/get` to provide standardized prompt templates for agent workflows.

#### Scenario: List available prompts
- **WHEN** client issues `prompts/list`
- **THEN** server returns an array containing `dev_workflow`, `debug_target`, `program_target`, and `analyze_bus` with name and description.

#### Scenario: Get specific prompt template
- **WHEN** client issues `prompts/get` with name `"dev_workflow"` and optional target parameter
- **THEN** server returns the parameterized prompt messages instructing the AI agent on closed-loop execution.
