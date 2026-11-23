# Proposal

## Why

Executing software bitbang SWD, flash routines, core halting, and register inspection over HTTP API or MCP tools causes resource contention with the dedicated hardware USB debug probe engines (CMSIS-DAP v1/v2, ST-Link v2, and WCH-Link), introduces Embassy executor latency, and lacks robust hardware flow control. Disabling programmer endpoints across both HTTP REST and MCP surfaces guarantees that all target debugging, memory access, halting, and flash programming are driven exclusively through native USB debug probe interfaces with host tools such as `probe-rs`, `pyocd`, `openocd`, and `st-flash`.

## What Changes

- **Disable Programmer Endpoints in API and MCP**:
  - Gated all `/api/v1/programmer/*` routes in `firmware-rs/src/rest.rs` to return the structured warning payload pointing to direct USB interfaces.
  - Gated all programmer-related tools in `firmware-rs/src/mcp.rs` (`probe_target`, `target_diagnose`, `inspect_firmware`, `programmer_read`, `programmer_flash`, `programmer_reset`, `programmer_unbrick`, `swo_read`) behind `HTTP_PROGRAMMER_ENABLED` to return structured warning payloads directing agents to use USB CMSIS-DAP, ST-Link, or WCH-Link.
- **Update MCP Tools Documentation**: Update descriptions of programmer tools in `crates/agentla-mcp/src/tools.rs` to state that programmer operations must be performed via direct host USB debug probes.
- **Update MCP Skills and Prompts**:
  - Update `agentla://skills/programmer`, `agentla://skills/dev`, and `agentla://skills/debug` in `crates/agentla-mcp/src/server.rs` to instruct agents to connect directly via host USB debug probes (`CMSIS-DAP`, `ST-Link`, `WCH-Link`).
  - Update `dev_workflow`, `debug_target`, and `program_target` prompt templates to mandate host USB debug probe toolchains (`probe-rs`, `pyocd`, `openocd`).
- **Retain Personality Switching**: Maintain the USB programmer personality selection mechanism so clients/agents can switch between CMSIS-DAP, ST-Link, and WCH-Link.

## Capabilities

### Modified Capabilities
- `rest-target-programmer`: Target programmer endpoints return structured warning payloads directing clients to direct USB debug probes.
- `agentla-programmer`: Programmer agent skill and MCP prompt sequences specify direct USB flashing and debugging workflows.

## Impact

- `firmware-rs/src/mcp.rs`: All programmer tool dispatch arms return structured warning payloads directing callers to direct USB debug probes.
- `crates/agentla-mcp/src/tools.rs`: Tool schemas updated with direct USB guidance.
- `crates/agentla-mcp/src/server.rs`: Skill texts and prompt templates updated.
- `firmware-rs/src/rest.rs`: Updated warning message.
