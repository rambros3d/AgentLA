## 1. Universal Skill Definition & Playbook

- [x] 1.1 Create `SKILLS/mcu-autonomous-testing/SKILL.md` with metadata, target architectures, prerequisites, and MCP tool mapping, packaged into LittleFS and served via HTTP & MCP
- [x] 1.2 Implement Phase 1: Electrical & Silicon Auto-Discovery workflow (`status`, `probe_target`, `target_diagnose`, `catalog_lookup`)
- [x] 1.3 Implement Phase 2: Vector Table Pre-Check & Verified Flash Deployment workflow (`inspect_firmware`, `programmer_flash`, `programmer_reset`)
- [x] 1.4 Implement Phase 3: Post-Reset Settling Delay (100 ms) & Console Handshake rules
- [x] 1.5 Implement Phase 4: Multi-Protocol Interactive Stimulus guidelines (`uart_command`, `can_send` / `can_sniff`, `i2c_scan`, `spi_transfer`)
- [x] 1.6 Implement Phase 5: Autonomous HardFault Triage & Root-Cause Synthesis decision tree (`target_diagnose` with CFSR/HFSR/BFAR decoding)

## 2. Reusable Test Runner & Report Formatting

- [x] 2.1 Implement `tools/mcu_test_runner.py` providing a programmatic closed-loop runner consuming AgentLA MCP JSON-RPC
- [x] 2.2 Implement TAP 13 (Test Anything Protocol) and Markdown summary reporting utilities

## 3. Automated Verification & Regression Tests

- [x] 3.1 Create test suite `test/test_mcu_autonomous_testing.py` verifying skill frontmatter, schema, LittleFS packaging, and step consistency
- [x] 3.2 Verify HardFault triage synthesis across Cortex-M (CFSR/HFSR/BFAR) and RISC-V targets
- [x] 3.3 Run full `pytest` suite and `cargo check --manifest-path firmware-rs/Cargo.toml` to verify zero regressions
