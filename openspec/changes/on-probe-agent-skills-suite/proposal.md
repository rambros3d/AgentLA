## Why

AI coding agents interacting with embedded target hardware through AgentLA need domain-specific knowledge, pinout constraints, and structured operational playbooks to function safely and autonomously. Without codified skills, AI agents frequently overlook electrical rail verification (damaging 1.8V/3.3V logic), guess GPIO pin assignments without acquiring ownership from the Pin Arbiter, fail to interpret Cortex-M HardFault status registers accurately, or cannot execute closed-loop verification when synthesizing firmware.

By embedding four specialized AI agent skills (`agentla-dev`, `agentla-debug`, `agentla-programmer`, `agentla-analyzer`) directly inside AgentLA's on-probe LittleFS partition and exposing them via Model Context Protocol (MCP) resources and prompts, any AI assistant (Antigravity, Claude Code, Cursor, Copilot) connected to AgentLA can immediately discover and follow battle-tested hardware workflows.

## What Changes

- **Author 4 Specialized On-Probe Skills** in `SKILLS/`:
  - `SKILLS/agentla-dev/SKILL.md`: Autonomous closed-loop firmware development, target-to-probe wiring instructions, code generation, automated flashing, and physical signal verification (e.g., verifying GPIO toggle frequencies with the logic analyzer).
  - `SKILLS/agentla-debug/SKILL.md`: Hardware triage and silicon fault analysis, 3.3V target rail verification, CoreSight DPIDR/CPUID scanning, AP discovery, and CFSR/HFSR/MMFAR HardFault root-cause decoding.
  - `SKILLS/agentla-programmer/SKILL.md`: Universal target flash programming, memory map catalog queries, mass erase operations, and vendor-specific rescue unbricking (STM32 option bytes, nRF APPROTECT, RP2040/RP2350 rescue).
  - `SKILLS/agentla-analyzer/SKILL.md`: High-speed bus capture and protocol decoding (I2C, SPI, UART, CAN, 1-Wire, DMX512), Pin Arbiter ownership enforcement, 120Ω bus termination relay toggling, and non-destructive packet injection.
- **MCP Skills & Prompts Surface**:
  - Expose `resources/list` entries for `agentla://skills/dev`, `agentla://skills/debug`, `agentla://skills/programmer`, and `agentla://skills/analyzer`.
  - Expose skill markdown content via `resources/read`.
  - Implement `prompts/list` and `prompts/get` supporting `dev_workflow`, `debug_target`, `program_target`, and `analyze_bus`.
- **WebUI Skills Playbook Viewer**:
  - Add an interactive "Agent Skills & Playbooks" modal or view in the WebUI to allow human engineers to inspect skill documentation, view required wiring topologies, and copy system prompts for external agent sessions.
  - Serve all skill files statically over HTTP at `/skills/<name>/SKILL.md`.
- **Packaging & Budget**:
  - Integrate skill file packaging into `tools/pack_webui.py`.
  - Verify that total uncompressed and compressed image size stays comfortably within the 512 KB LittleFS static partition limit.

## Capabilities

### New Capabilities
- `agentla-dev`: Autonomous closed-loop firmware developer playbook providing wiring guidance, code synthesis, flashing, and logic analyzer physical verification.
- `agentla-debug`: Hardware triage, rail voltage verification, CoreSight AP discovery, and fault register root-cause analysis playbook.
- `agentla-programmer`: Universal target flash programming, memory map catalog lookup, mass erase, and unbricking playbook.
- `agentla-analyzer`: Bus protocol sniffing, signal decoding, Pin Arbiter contract compliance, and packet injection playbook.
- `on-probe-skills-storage`: Static hosting and interactive WebUI presentation of on-probe skills within LittleFS.

### Modified Capabilities
- `mcp-protocol-surface`: Expose `agentla://skills/*` resources and `prompts/list` + `prompts/get` methods in the MCP JSON-RPC server.

## Impact

- `crates/agentla-mcp`: Adds prompt listing/fetching and skill resource discovery to `#![no_std]` zero-heap JSON-RPC engine.
- `SKILLS/`: Introduces directory structure and markdown files for the 4 skills alongside `mcu-autonomous-testing`.
- `webui/apps.html`: Adds "Agent Skills & Playbooks" tab or drawer with one-click copy prompt capability.
- `tools/pack_webui.py`: Automatically includes `skills/` during LittleFS image generation.
- ROM/Flash Footprint: Adds < 6 KB compressed to the static partition (currently 215 KB / 512 KB, ~300 KB headroom).
