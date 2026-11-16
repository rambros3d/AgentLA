## Why

AgentLA features an on-device Model Context Protocol (MCP) server providing 29 low-level hardware instrumentation and bus primitives. However, external AI agents currently have no MCP interface to query or install LittleFS user applications, dynamically control or power-cycle target voltage rails, trigger an atomic hardware safe-state, or switch hardware profile slots. 

Exposing these high-value capabilities over MCP empowers AI agents to autonomously manage applications, run automated target firmware test cycles, and safely protect hardware benches, while enforcing safety guardrails by explicitly reserving app uninstallation for manual user interaction only.

## What Changes

- Add Safe Application Management MCP Tools in `crates/agentla-mcp` and `firmware-rs/src/mcp.rs`:
  - `app_list`: Query installed applications, version manifests, and LittleFS storage capacity.
  - `app_install`: Install or update a user application from the local catalog or incoming package data.
  - *Safety Guardrail*: `app_uninstall` is strictly omitted from MCP and restricted to manual WebUI / LittleFS operations.
- Add Power & Safety Control MCP Tools:
  - `power_control`: Enable or disable the target power rail switch and set output voltage.
  - `power_cycle`: Pulse the target power rail off and back on with a configurable millisecond delay to reboot targets.
  - `emergency_safe_state`: Atomically transition all GPIO and PWM pins to High-Z and open the power rail switch.
- Add Profile & Trace MCP Tools:
  - `profile_list`: Enumerate hardware profile slots (Slot 0 Safe State, Slots 1-7 User Profiles).
  - `profile_apply`: Switch active hardware profile slot without manual pin reconfiguration.
  - `swo_read`: Retrieve captured Serial Wire Output (SWO) trace buffer bytes from Cortex-M debug sessions.
- Update `webui/mcp.json` tool count and schema documentation.

## Capabilities

### New Capabilities
- `mcp-app-management`: Safe application discovery and installation over MCP with uninstallation restricted to manual operations.
- `mcp-power-and-safety`: Target power rail switching, reboot power-cycling, and emergency safe-state execution over MCP.
- `mcp-profile-and-trace`: Flash hardware profile slot management and target SWO trace inspection over MCP.

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `crates/agentla-mcp/`: Tool definitions and static JSON input schemas expanded.
- `firmware-rs/src/mcp.rs`: Dispatch match arms added for new tools.
- `webui/mcp.json`: Tool count incremented to reflect added capabilities.
- Zero breaking changes to existing 29 MCP tools or REST endpoints.
