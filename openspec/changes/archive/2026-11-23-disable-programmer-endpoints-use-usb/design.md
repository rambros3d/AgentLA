# Design

## Context

See `proposal.md` for motivation.

AgentLA exposes dedicated hardware USB endpoints for debug probes:
- CMSIS-DAP v1/v2 (works natively with `probe-rs`, `pyocd`, Keil, IAR)
- ST-Link v2 (works natively with `st-flash`, OpenOCD, STM32CubeProgrammer)
- WCH-Link (works with WCH RISC-V tools for CH32V targets)

Previously, only flash programming was gated in MCP, while tools like `probe_target`, `target_diagnose`, `inspect_firmware`, `programmer_reset`, and `swo_read` directly executed bitbang SWD inside Embassy async tasks. This caused hardware bus contention and timing jitter with active USB debug sessions.

## Goals / Non-Goals

**Goals:**
- Gate all programmer tools in `firmware-rs/src/mcp.rs` (`probe_target`, `target_diagnose`, `inspect_firmware`, `programmer_read`, `programmer_flash`, `programmer_reset`, `programmer_unbrick`, `swo_read`) behind `HTTP_PROGRAMMER_ENABLED` so they return a structured warning payload pointing to direct USB interfaces.
- Update `crates/agentla-mcp/src/tools.rs` descriptions to clearly state that target debugging, memory reading, and flashing must be performed via direct USB debug probes (`CMSIS-DAP`, `ST-Link`, `WCH-Link`).
- Update `crates/agentla-mcp/src/server.rs` skill playbooks (`agentla://skills/programmer`, `agentla://skills/dev`, `agentla://skills/debug`) and prompts (`dev_workflow`, `debug_target`, `program_target`) to direct agents to use host USB debug probe toolchains.
- Retain USB personality switching so host agents can select between CMSIS-DAP, ST-Link, and WCH-Link modes.

**Non-Goals:**
- Removing the CMSIS-DAP, ST-Link, or WCH-Link USB engines: these remain fully functional and are the intended primary interfaces.

## Decisions

### 1. Consistent Warning Payloads Across API and MCP
- **Decision**: Return a structured warning JSON object with `status: "disabled"`, `use: "usb"`, and `modes: ["cmsis", "stlink", "wch"]`.
- **Rationale**: Gives AI agents deterministic, actionable next-steps without causing unhandled exceptions.

### 2. Comprehensive MCP Programmer Tool Gating
- **Decision**: Gating encompasses not just flash writes, but target detection (`probe_target`), diagnostics (`target_diagnose`), vector table inspection (`inspect_firmware`), target reset (`programmer_reset`), unbrick (`programmer_unbrick`), and trace reading (`swo_read`).
- **Rationale**: Eliminates all concurrent SWD bitbang on the shared debug pins while the hardware USB debug probe interface is active.

## Risks / Trade-offs

- **[Risk] Existing agent scripts invoking `probe_target` or `target_diagnose` receive disabled responses.**  
  → **Mitigation**: The response clearly provides the compatible USB modes and instructs the agent to run `probe-rs info` or `pyocd` on the host USB interface.
