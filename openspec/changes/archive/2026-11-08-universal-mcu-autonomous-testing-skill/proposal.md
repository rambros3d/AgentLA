## Why

Autonomous AI coding agents (such as Antigravity, Claude, and Cursor) possess deep reasoning capabilities but lack structured methodology when executing hardware tests across diverse microcontroller targets. While AgentLA exposes 29 low-level MCP primitives (such as `programmer_flash`, `uart_command`, and `target_diagnose`), an agent cold-starting on a board cannot autonomously coordinate the end-to-end closed loop: discovering chip architecture, verifying electrical rails, flashing firmware, waiting out boot settling times, injecting bus stimulus, and triaging CPU HardFaults when tests fail.

Creating a dedicated, universal `mcu-autonomous-testing` skill bridges the gap between raw MCP tools and autonomous agent behavior, giving any AI agent a standardized, reproducible playbook for testing any MCU architecture (ARM Cortex-M, RISC-V, etc.) with zero human intervention.

## What Changes

- Create a universal agent skill in `.agent/skills/mcu-autonomous-testing/SKILL.md` defining the closed-loop autonomous testing workflow for AI agents.
- Standardize multi-stage test orchestration:
  1. **Phase 1: Auto-Discovery & Health Audit** (`status`, `probe_target`, `target_diagnose`, `catalog_lookup`): Verifies target voltage rails ($\ge 2.7\text{V}$), inspects pin continuity, identifies silicon family and SRAM/Flash boundaries without destructive operations.
  2. **Phase 2: Firmware Deployment & Validation** (`inspect_firmware`, `programmer_flash`, `programmer_reset`): Validates vector table Thumb-bit and Initial SP invariants before streaming verified flash blocks.
  3. **Phase 3: Interactive Stimulus & Observation** (`uart_command`, `can_send` / `can_sniff`, `i2c_scan`, `spi_transfer`, `logic_analyzer_*`): Stimulates target interfaces and verifies responses against expected test patterns.
  4. **Phase 4: Fault Triage & Root-Cause Synthesis** (`target_diagnose`): Automatically interrogates CoreSight SCB (CFSR, HFSR, BFAR) on timeouts or abnormal responses to synthesize the exact failure reason (e.g. Unaligned access, BusFault, Lockup).
  5. **Phase 5: Standardized Result Reporting**: Emits structured test verdicts (PASS/FAIL/SKIP) with timing and electrical telemetry.
- Provide reusable Python test harness scripts and mock verification suites in `test/test_mcu_autonomous_testing.py`.

## Capabilities

### New Capabilities
- `universal-mcu-autonomous-testing`: Closed-loop autonomous hardware test execution, architecture auto-discovery, interactive bus stimulus, and root-cause fault triage skill for AI agents using AgentLA MCP.

### Modified Capabilities
<!-- None: existing capability specifications remain unchanged -->

## Impact

- **Skills**: Adds `.agent/skills/mcu-autonomous-testing/SKILL.md` (instructions, tool invocation rules, and diagnostic trees).
- **Tool Consumption**: Standardizes agent usage of `crates/agentla-mcp` tools (`target_diagnose`, `probe_target`, `catalog_lookup`, `programmer_flash`, `programmer_reset`, `uart_command`, `sniff_bus`, `pins_claim`).
- **Tests**: Adds unit tests verifying skill format, step sequence validation, and fault diagnosis decision trees.
