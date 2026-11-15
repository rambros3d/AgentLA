## Context

AgentLA is an RP2350-based hardware bench tool running an Embassy `#![no_std]` async stack, hosting high-speed logic analyzer DMA streaming, multi-target SWD flash programming, protocol decoders, and a Model Context Protocol (MCP) JSON-RPC 2.0 endpoint. 

When external AI agents (such as Antigravity, Claude Code, Cursor, or ChatGPT) connect to AgentLA over MCP or HTTP, they require standardized domain guidance to interact safely with external physical circuits. Lacking explicit playbooks, agents frequently make common embedded mistakes: driving SWD lines before verifying target power (VTGT), ignoring pin conflicts without reserving pins in the Pin Arbiter, misidentifying Cortex-M HardFault bits, or declaring firmware "tested" without physical bus verification.

AgentLA maintains a dedicated 512 KB LittleFS static web partition (`webui/`), currently utilizing ~210 KB with >300 KB free headroom.

## Goals / Non-Goals

**Goals:**
- Provide 4 complete, battle-tested `SKILL.md` documents in `SKILLS/`:
  1. `SKILLS/agentla-dev/SKILL.md`: Autonomous closed-loop firmware developer/debugger.
  2. `SKILLS/agentla-debug/SKILL.md`: Hardware triage, rail safety, CoreSight discovery, and HardFault root-cause analysis.
  3. `SKILLS/agentla-programmer/SKILL.md`: Universal target flashing, memory map catalog lookup, mass erase, and unbricking.
  4. `SKILLS/agentla-analyzer/SKILL.md`: High-speed bus capture, protocol decoding, 120Ω relay control, and safe packet injection.
- Expose skills natively over MCP via `resources/list`, `resources/read`, and `prompts/list` / `prompts/get`.
- Serve skill documents statically over HTTP at `/skills/<name>/SKILL.md`.
- Add an interactive "Agent Skills & Playbooks" section to the WebUI allowing engineers to inspect playbooks and copy one-click prompts.
- Maintain strict `#![no_std]` zero-heap compliance in `crates/agentla-mcp`.
- Keep WebUI partition consumption under 360 KB (leaving >160 KB headroom).

**Non-Goals:**
- Running on-probe LLM inference or weights.
- Replacing external compiler toolchains (compilation is orchestrated by the agent in its host environment; AgentLA executes the flashing and physical verification).

## Decisions

### 1. Dual-Access Storage Pattern (HTTP + MCP)
- **Choice**: Store skill markdown files in `SKILLS/<skill-name>/SKILL.md`.
- **Rationale**: Files are discovered from `SKILLS/` and bundled into the static LittleFS filesystem by `tools/pack_webui.py`. This makes them instantly accessible over HTTP GET (`http://10.10.10.10/skills/...`), usable in the browser WebUI, and callable via MCP resources.
- **Alternatives Considered**: 
  - Embedding entire markdown strings as `&'static str` in Rust firmware binary: Wastes RP2350 Flash ROM code space. LittleFS static partition has 301 KB unused space.

### 2. MCP Prompts vs Resources Architecture
- **Choice**: Expose both MCP Resources (`agentla://skills/*`) and MCP Prompts (`dev_workflow`, `debug_target`, `program_target`, `analyze_bus`).
- **Rationale**: MCP Resources allow RAG tools and agents to inspect skill documents on demand. MCP Prompts allow agent clients (like Claude Desktop or Cursor) to present interactive action buttons that preload the exact hardware workflow instructions into the conversation context.

### 3. Closed-Loop Developer Protocol in `agentla-dev`
- **Choice**: Enforce an 8-phase state machine:
  1. Target Catalog Query (`catalog_lookup`)
  2. User Wiring Instructions (SWD + VTGT + Test Pin)
  3. Pre-Flight Power & Target Detection (`mcp_target_diagnose`)
  4. Firmware Binary Synthesis / Compilation
  5. Target Programming (`programmer_flash` + verification)
  6. Pin Arbiter Acquisition (`pins_claim`)
  7. Physical Signal Verification (Logic Analyzer capture & pulse timing verification)
  8. Autonomous Self-Healing / Completion
- **Rationale**: Eliminates the "silent failure" problem where firmware flashes successfully but hangs in an unhandled interrupt or clock fault.

### 4. Zero-Heap MCP Response Serialization
- **Choice**: Implement `prompts/list`, `prompts/get`, and `resources/list` in `crates/agentla-mcp` using `core::fmt::Write` streaming directly into heapless string buffers.
- **Rationale**: Complies with strict `#![no_std]` zero-heap architectural rule.

## Risks / Trade-offs

- **[Risk] LittleFS Partition Overrun**: Adding rich documentation might inflate the LittleFS partition.
  - *Mitigation*: The 4 markdown files together are ~15 KB raw and compress with heatshrink/deflate to < 4 KB. Headroom is >300 KB.
- **[Risk] Pin Contention During Automated Loop**: If the agent attempts a logic analyzer capture while another subsystem holds the pin, the verification would crash.
  - *Mitigation*: All skill playbooks explicitly mandate calling `pins_claim` before sampling and handling `isError` gracefully.
