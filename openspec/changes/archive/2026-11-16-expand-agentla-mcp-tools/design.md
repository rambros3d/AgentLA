## Context

AgentLA's `agentla-mcp` crate and `firmware-rs/src/mcp.rs` implement a zero-allocation `#![no_std]` Model Context Protocol (MCP) JSON-RPC 2.0 server over HTTP (`POST /mcp`) and SSE. It currently exposes 29 tools focused on pin reading, hardware bus transfers, logic analyzer capture, and target programming.

Currently, several high-value firmware facilities are accessible only through REST endpoints or frontend JavaScript:
1. LittleFS application listing and tarball unpacking.
2. Target power rail switching, voltage DAC adjustment, and power-cycling.
3. Hardware emergency safe-state pin release.
4. Non-volatile hardware profile slot switching.
5. Cortex-M SWO ITM trace buffer reading.

This design extends the MCP tool registry to bridge these facilities to external AI agents while explicitly enforcing a safety guardrail that prohibits app uninstallation over MCP.

## Goals / Non-Goals

**Goals:**
- Add `app_list` and `app_install` to allow autonomous discovery and installation of official catalog applications without requiring raw HTTP tarball uploads.
- Add `power_control` and `power_cycle` to give agents programmatic control of the target power rail for automated reset and brownout testing.
- Add `emergency_safe_state` providing a one-shot atomic safety trigger that trips all pin drivers into High-Z and cuts target rail power.
- Add `profile_list` and `profile_apply` to enable agents to toggle test-bench pin configurations atomically across flash profile slots.
- Add `swo_read` to allow agents to inspect target firmware debug logs directly over MCP.
- Maintain zero heap allocation in `agentla-mcp` and bounded response buffer sizes.
- Adhere strictly to AgentLA guidelines (zero emojis, `#![no_std]` compliance).

**Non-Goals:**
- Providing an `app_uninstall` tool over MCP. App deletion is intentionally restricted to physical human interaction via the WebUI desktop or direct filesystem administration.
- Replacing or deprecating existing REST endpoints or the 29 existing MCP tools.

## Decisions

### D1: Exclude `app_uninstall` from MCP
- **Decision**: Provide `app_list` and `app_install`, but intentionally omit `app_uninstall`.
- **Rationale**: Prevents accidental data loss or cascading deletions by autonomous agents. If an app must be removed, the user must explicitly do so from the AgentLA desktop UI or LittleFS file manager.

### D2: Reuse Existing Firmware Facilities
- **Decision**: Map MCP tools directly to internal functions already maintained in `firmware-rs`:
  - `app_list` maps to `crate::apps::list_apps_json`.
  - `power_control` and `power_cycle` map to `crate::profile::set_target_power` and `crate::telemetry::set_target_voltage_mv`.
  - `emergency_safe_state` maps to `crate::profile::apply_profile(0)` and pin arbiter teardown.
  - `profile_list` and `profile_apply` map to `crate::profile::apply_profile` and slot queries.
  - `swo_read` maps to `crate::programmer::swo::drain_swo_bytes`.
- **Rationale**: Minimizes code footprint and avoids duplicating state machines in firmware.

### D3: Bounded In-Place JSON Formatting
- **Decision**: Tools format output directly into the caller's bounded heapless string buffer (`heapless::String<1536>`), using existing argument extractors (`extract_string_arg`, `extract_u32_arg`, `extract_bool_arg`).
- **Rationale**: Preserves `#![no_std]` zero-heap guarantees and ensures predictable interrupt/executor latency.

## Risks / Trade-offs

- **[Risk] High-capacity app tarball streaming blocking MCP request thread**
  - **Mitigation**: `app_install` accepts a catalog app ID (e.g. `"barefoc"`, `"servo"`, `"canbus"`) which resolves against cached system package bundles, or small single-file bundles.
- **[Risk] Sudden target power loss causing bus faults**
  - **Mitigation**: `emergency_safe_state` transitions claimed pins to High-Z before opening the power switch to prevent back-powering targets through GPIO protection diodes.
