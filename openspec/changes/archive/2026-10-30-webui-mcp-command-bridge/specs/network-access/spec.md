## MODIFIED Requirements

### Requirement: Probe-side network access
The system SHALL serve an actual command-capable network interface from the RP2350 over USB-RNDIS, including structured command request and result channels, command handle support, and method discovery.

#### Scenario: Agent connects
- **WHEN** LLM agent connects to probe IP (pico.lab/agentla.lab)
- **THEN** `/mcp` SHALL support discovery of allowed command tools and execute permitted commands

#### Scenario: Browser calls command
- **WHEN** a WebUI sends a valid JSON-RPC command request
- **THEN** firmware SHALL route it to the existing command dispatcher and return a structured response

## ADDED Requirements

### Requirement: Dual-core capability remains behaviorally consistent
The system SHALL document whether network command execution and capture may share a core, and SHALL avoid claiming passive asynchronous operation without a tested core-split implementation.

#### Scenario: Implementation status
- **WHEN** network features are verified
- **THEN** the definition of dual-core operation SHALL reflect an actual launched Core 1 or a revised single-core contract