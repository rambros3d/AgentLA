## Why

While AgentLA recently promoted Servo Master to a native in-ROM system app and implements register-level PWM servo control (`crate::servo`), the Model Context Protocol (MCP) server currently only exposes `servo_list`. Autonomous AI agents interacting with AgentLA via MCP cannot directly configure or actuate servos (attach, position angle, throttle continuous rotation, pulse width in microseconds, or detach).

Extending the MCP tool surface with full servo actuation tools empowers AI agents to autonomously control robotic joints, camera gimbals, pan/tilt mechanisms, and RC actuators directly through standard JSON-RPC tool calls.

## What Changes

- Update `crates/agentla-mcp/src/tools.rs`:
  - Define `TOOL_SERVO_ATTACH` (`servo_attach`): Attach a GPIO pin with optional min/max pulse limits and continuous mode.
  - Define `TOOL_SERVO_WRITE_ANGLE` (`servo_write_angle`): Command target angle (0 to 180 degrees) or continuous rotation throttle (-100% to +100%).
  - Define `TOOL_SERVO_WRITE_US` (`servo_write_us`): Command direct pulse duration in microseconds (clamped to channel limits).
  - Define `TOOL_SERVO_DETACH` (`servo_detach`): Stop PWM generation and release the GPIO pin back to the arbiter.
  - Register all four tools in `ALL_TOOLS`.
- Update `firmware-rs/src/mcp.rs`:
  - Implement JSON-RPC tool dispatch handlers for `servo_attach`, `servo_write_angle`, `servo_write_us`, and `servo_detach`, delegating to `crate::servo`.
- Update `webui/mcp.json`:
  - Increment tool count from 36 to 40.
- Add test coverage verifying tool declaration, schema validity, and dispatch.

## Capabilities

### New Capabilities
- `mcp-servo-controls`: Exposes full servo channel lifecycle management and pulse-width/angle actuation tools via MCP JSON-RPC 2.0.

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `crates/agentla-mcp`: Adds 4 tool definitions and exports in `ALL_TOOLS`.
- `firmware-rs/src/mcp.rs`: Adds handler branches under `handle_tool`.
- `webui/mcp.json`: Updates tools count to 40.
- AI Agents: Enables full multi-channel servo actuation via MCP.
