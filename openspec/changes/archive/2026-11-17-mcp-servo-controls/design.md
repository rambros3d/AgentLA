## Context

AgentLA implements a monolithic `#![no_std]` MCP server (`crates/agentla-mcp` and `firmware-rs/src/mcp.rs`) accessible via HTTP POST at `/mcp` and Server-Sent Events at `/mcp/sse`. The tool surface already exposes instruments for logic analysis, SWD flashing, protocol decoding, GPIO pins, and power control, but only exposes read-only channel listing (`servo_list`) for servos.

The underlying hardware engine (`firmware-rs/src/servo.rs`) already implements safe pin arbitration, 50 Hz PWM timebase generation, positional angle computation (0-180 deg), continuous rotation throttle mapping (-100% to +100%), and microsecond clamping.

This design wires the existing `crate::servo` functions into MCP tool declarations and JSON-RPC dispatchers.

## Goals / Non-Goals

**Goals:**
- Declare 4 new MCP tools in `crates/agentla-mcp/src/tools.rs`:
  - `servo_attach`
  - `servo_write_angle`
  - `servo_write_us`
  - `servo_detach`
- Implement robust parameter extraction (pin index, min/max pulse limits, angle, pulse microseconds, continuous flag) in `firmware-rs/src/mcp.rs`.
- Handle pin claiming errors, slice conflicts, and range limits cleanly, returning informative error messages with `isError: true`.
- Return structured status summaries (active pin, current angle, pulse_us, active state) on successful tool calls.
- Update `webui/mcp.json` manifest tool count to 40.
- Maintain `#![no_std]` and zero emojis.

**Non-Goals:**
- Changing existing REST endpoints (`/api/v1/servo/*`).
- Modifying the underlying register PWM math in `servo.rs`.

## Decisions

### D1: Tool Naming and Schema Conventions
- **Decision**: Prefix with `servo_` (`servo_attach`, `servo_write_angle`, `servo_write_us`, `servo_detach`).
- **Rationale**: Consistent with existing `servo_list` and domain prefixing across the MCP tool registry (e.g. `logic_analyzer_*`, `programmer_*`, `can_*`).

### D2: Reuse Existing Argument Parsing Helpers
- **Decision**: Utilize existing lightweight JSON-RPC extraction utilities (`extract_u32_arg`, `extract_i32_arg`, `extract_bool_arg` or equivalent) in `firmware-rs/src/mcp.rs`.
- **Rationale**: Prevents code bloat and guarantees zero-heap stack-bounded execution on the RP2350.

### D3: Safe Pin Arbitration Feedback
- **Decision**: Return descriptive errors when `servo::attach` fails due to `EngineError::SliceConflict` or `pin_owner` reservation conflicts.
- **Rationale**: Agents need actionable feedback when an adjacent pin or slice is already in use by an incompatible frequency or peripheral.

## Risks / Trade-offs

- **[Risk] Pin / Slice Conflicts**: RP2350 PWM slices are paired (GP0/1 share slice 0, GP2/3 share slice 1, etc.).
  - **Mitigation**: `crate::servo` already checks slice frequency compatibility. If a slice conflict occurs, the MCP handler reports `isError: true` with a clear explanation.
