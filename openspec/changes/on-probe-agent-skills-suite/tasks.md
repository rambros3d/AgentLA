## 1. Skill Definitions Authoring

- [x] 1.1 Author `SKILLS/agentla-dev/SKILL.md` with closed-loop autonomous developer playbook, wiring guidance, compile & flash sequence, pin arbiter acquisition, and logic analyzer physical verification.
- [x] 1.2 Author `SKILLS/agentla-debug/SKILL.md` with target rail voltage check, CoreSight DPIDR/CPUID discovery, AP scanning, and HardFault CFSR/HFSR/MMFAR decoding.
- [x] 1.3 Author `SKILLS/agentla-programmer/SKILL.md` with chip catalog lookup, flash boundary validation, chunked flash programming, readback verification, and rescue unbrick sequences.
- [x] 1.4 Author `SKILLS/agentla-analyzer/SKILL.md` with Pin Arbiter reservation contract, high-speed logic analyzer capture, bus decoding (I2C/SPI/UART/CAN), 120Ω relay control, and packet injection.

## 2. MCP Server Protocol Surface

- [ ] 2.1 Update `crates/agentla-mcp/src/server.rs` to expose `agentla://skills/dev`, `agentla://skills/debug`, `agentla://skills/programmer`, and `agentla://skills/analyzer` in `resources/list` and `resources/read`.
- [ ] 2.2 Implement `prompts/list` and `prompts/get` in `crates/agentla-mcp/src/server.rs` supporting `dev_workflow`, `debug_target`, `program_target`, and `analyze_bus`.
- [ ] 2.3 Add tests in `crates/agentla-mcp` verifying prompt listing, prompt retrieval, and skill resource serialization under `#![no_std]`.

## 3. WebUI Skills Console

- [ ] 3.1 Update `webui/apps.html` (or dedicated view) to present an "Agent Skills & Playbooks" drawer/tab with interactive skill cards, wiring cheat sheets, and "Copy Prompt" action buttons.
- [ ] 3.2 Verify styling and responsiveness of the Skills Console matching the existing DaisyUI/Tailwind dark theme.

## 4. Packaging, Live Verification & Deployment

- [x] 4.1 Update `tools/pack_webui.py` to ensure `SKILLS/` is packaged into LittleFS and verify that total compressed size remains well under 512 KB.
- [ ] 4.2 Run `cargo check` and `cargo test` across crates to confirm zero compilation errors or regressions.
- [ ] 4.3 Deploy the updated WebUI and skills bundle to live hardware at `http://10.10.10.10/api/v1/system/webui/update` and verify retrieval via HTTP and MCP.
